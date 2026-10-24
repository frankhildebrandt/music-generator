use super::*;
use music_core::{
    audio::{Audio, RATE},
    loops::{EditRecipe, GenerationRequest, DEFAULT_MODEL, REQUIRED_MODELS},
    types::id,
};
pub struct Job {
    pub instance: String,
    pub cancel: CancellationToken,
    pub value: Value,
}
async fn create(service: &Service, instance: String) -> Result<(String, CancellationToken)> {
    let mut jobs = service.jobs.lock().await;
    if jobs
        .values()
        .filter(|j| ["queued", "running"].contains(&j.value["status"].as_str().unwrap_or("")))
        .count()
        >= 64
    {
        return Err(message("Generation queue is full"));
    }
    if jobs.len() > 256 {
        jobs.retain(|_, j| {
            ["queued", "running"].contains(&j.value["status"].as_str().unwrap_or(""))
        });
    }
    let jid = id();
    let token = CancellationToken::new();
    jobs.insert(
        jid.clone(),
        Job {
            instance,
            cancel: token.clone(),
            value: json!({"id":jid,"status":"queued","message":"Queued","results":[]}),
        },
    );
    Ok((jid, token))
}
async fn update(service: &Service, jid: &str, value: Value) {
    if let Some(job) = service.jobs.lock().await.get_mut(jid) {
        if !job.cancel.is_cancelled() {
            job.value = value;
        }
    }
}
pub async fn enqueue_install(service: Arc<Service>, instance: String) -> Result<Value> {
    let (jid, cancel) = create(&service, instance).await?;
    let result = json!({"id":jid});
    let task = async move {
        let outcome=async {
            let _permit=tokio::select!{_=cancel.cancelled()=>return Err(music_core::error::Error::Cancelled),p=service.serial.acquire()=>p.map_err(|e|message(e.to_string()))?};
            update(&service,&jid,json!({"id":jid,"status":"running","message":"Checking models"})).await;
            let root=service.models.lock().await.clone();let mut verifier=music_core::verification::ModelVerifier::open(&service.root);let client=reqwest::Client::new();let manifest=music_core::manifest::manifest();
            for model in manifest.models.into_iter().filter(|m|REQUIRED_MODELS.contains(&m.file.as_str())) {
                music_core::download::download_model(&client,&format!("{}/{}",manifest.base_url,model.file),&root,&model,&mut verifier,&cancel,|p|{
                    if let Ok(mut jobs)=service.jobs.try_lock(){if let Some(j)=jobs.get_mut(&jid){if !j.cancel.is_cancelled(){j.value=json!({"id":jid,"status":"running","message":format!("{} {}: {} / {} MiB",p.phase,p.file,p.downloaded/1048576,p.total/1048576)});}}}
                }).await?;
            }
            Ok::<_,music_core::error::Error>(json!({"id":jid,"status":"done","message":format!("Models ready · {}",root.display()),"results":[]}))
        }.await;
        finish(&service, &jid, outcome).await;
    };
    tokio::spawn(task);
    Ok(result)
}
pub async fn enqueue_generation(
    service: Arc<Service>,
    instance: String,
    mut request: GenerationRequest,
) -> Result<Value> {
    // Snapshot audio before enqueuing, so subsequent edits cannot alter the job.
    let snapshot = service.root.join("snapshots").join(id());
    std::fs::create_dir_all(&snapshot)?;
    for (n, source) in request.context.iter_mut().enumerate() {
        let path = PathBuf::from(&source.path);
        if path.metadata()?.len() > 256 * 1024 * 1024 {
            return Err(message("Context exceeds 256 MiB"));
        }
        let target = snapshot.join(format!("{n}.wav"));
        tokio::fs::copy(path, &target).await?;
        source.path = target.to_string_lossy().into();
    }
    let (jid, cancel) = create(&service, instance).await?;
    let result = json!({"id":jid});
    std::fs::write(
        snapshot.join("request.json"),
        serde_json::to_vec_pretty(&request)?,
    )?;
    let task = async move {
        let outcome=async {
            let _permit=tokio::select!{_=cancel.cancelled()=>return Err(music_core::error::Error::Cancelled),p=service.serial.acquire()=>p.map_err(|e|message(e.to_string()))?};
            update(&service,&jid,json!({"id":jid,"status":"running","message":"Starting ACE-Step"})).await;
            let mut guard=service.engine.lock().await;
            if guard.as_mut().is_some_and(|e|!e.alive()){guard.take();}
            if guard.is_none(){*guard=Some(engine::Engine::start(&service,&cancel).await?);}
            let engine=guard.as_ref().unwrap();
            let duration=request.seconds();
            let bar_seconds=request.numerator as f64*4.0/request.denominator as f64*60.0/request.bpm;
            let generated_duration=(duration+2.0*bar_seconds).clamp(10.0,600.0);
            let frames=(generated_duration*RATE as f64).round() as usize;
            let sources=request.context.clone();
            let context=tokio::task::spawn_blocking(move||->Result<Audio>{
                let mut mixed=Audio::silence(frames as u64);
                let mut budget=0;
                for source in sources {
                    let audio=Audio::read(std::path::Path::new(&source.path))?;budget+=audio.samples.len();
                    if budget>RATE as usize*60*12{return Err(message("Context exceeds twelve stereo track-minutes"));}
                    for (i,pair) in mixed.samples.iter_mut().enumerate(){for (ch, sample) in pair.iter_mut().enumerate(){*sample+=audio.samples[i%audio.samples.len()][ch];}}
                }
                mixed.headroom(0.8);Ok(mixed)
            }).await.map_err(|e|message(e.to_string()))??;
            let source=if context.peak()>1e-6{let path=snapshot.join("context.wav");context.write(&path)?;Some(path)}else{None};drop(context);
            let mut results=vec![];
            for variant in 0..request.variants {
                if cancel.is_cancelled(){return Err(music_core::error::Error::Cancelled);}
                let seed=if request.seed<0 {seconds() as i64+variant as i64}else{request.seed+variant as i64};
                let params=synthesis_params(&request,generated_duration,seed,source.is_some());
                let log_path=service.root.join("engine.log");
                let log_offset=log_path.metadata().map(|m|m.len()).unwrap_or(0);
                update(&service,&jid,json!({"id":jid,"status":"running","message":"Loading XL / encoding context","variant":variant,"variants":request.variants,"progress":null,"results":results})).await;
                let bytes=engine.client.generate_stem(&params,source.as_deref(),&cancel,|text|{
                    if let Ok(mut jobs)=service.jobs.try_lock(){if let Some(j)=jobs.get_mut(&jid){if !j.cancel.is_cancelled(){let step=crate::progress::synthesis_step(&log_path,log_offset);
                    j.value["message"]=json!(match step {Some((n,total))=>format!("XL · step {n}/{total} · {text}"),None=>format!("Loading XL / encoding context · {text}")});
                    j.value["progress"]=step.map(|(n,total)|json!(n as f64/total as f64*0.9)).unwrap_or(Value::Null);
                    j.value["step"]=step.map(|(n,_)|json!(n)).unwrap_or(Value::Null);
                    j.value["steps"]=step.map(|(_,total)|json!(total)).unwrap_or(Value::Null);}}}
                }).await?;
                update(&service,&jid,json!({"id":jid,"status":"running","message":"Preparing exact loop / waveform","variant":variant,"variants":request.variants,"progress":0.95,"results":results})).await;
                let audio=Audio::decode(&bytes)?;let target_frames=(duration*RATE as f64).round() as u64;
                if (audio.samples.len() as u64)<target_frames{return Err(message("Engine returned too little audio; existing loop preserved"));}
                let dir=service.root.join("assets");std::fs::create_dir_all(&dir)?;let original=dir.join(format!("{}-original.wav",id()));audio.write(&original)?;
                let (start,end)=music_core::loops::loop_region(&audio,target_frames,bar_seconds*RATE as f64,false)?;
                let recipe=EditRecipe{start,end,frames:target_frames,crossfade_ms:5.0,..Default::default()};
                let edited=music_core::loops::edit(&audio,&recipe)?;
                let path=dir.join(format!("{}.wav",id()));edited.write(&path)?;
                results.push(json!({"path":path,"original":original,"frames":target_frames,"bpm":request.bpm,"beats":request.beats,"bars":request.beats/(request.numerator as f64*4.0/request.denominator as f64),"key":request.key,"name":request.instrument,"seed":seed,"recipe":recipe,"sound_idea":request.sound_idea}));
                update(&service,&jid,json!({"id":jid,"status":"running","message":"Loop ready","variant":variant,"variants":request.variants,"progress":1.0,"results":results})).await;
            }
            Ok(json!({"id":jid,"status":"done","message":"Ready to audition — choose a variant","results":results}))
        }.await;
        finish(&service, &jid, outcome).await;
        let _ = std::fs::remove_dir_all(snapshot);
    };
    tokio::spawn(task);
    Ok(result)
}
async fn finish(service: &Service, jid: &str, outcome: Result<Value>) {
    let value = match outcome {
        Ok(v) => v,
        Err(e) => json!({"id":jid,"status":"error","message":e.to_string(),"results":[]}),
    };
    update(service, jid, value).await;
}

fn synthesis_params(request: &GenerationRequest, duration: f64, seed: i64, context: bool) -> Value {
    json!({
        "task_type": if context { "lego" } else { "text2music" },
        "track": request.instrument,
        "caption": format!("Continuous isolated {} loop, steady groove, no intro, no outro, no fade-out. {}", request.instrument, request.prompt),
        "lyrics": "[Instrumental]",
        "bpm": request.bpm,
        "duration": duration,
        "keyscale": request.key,
        "timesignature": request.numerator.to_string(),
        "seed": seed,
        "inference_steps": request.steps,
        "guidance_scale": 1.0,
        "shift": 1.0,
        "output_format": "wav32",
        "synth_model": DEFAULT_MODEL
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xl_is_used_for_text_and_context_requests_and_model_setup() {
        let request: GenerationRequest = serde_json::from_value(json!({
            "slot":36,"instrument":"drums","prompt":"Dry groove", "key":"A minor",
            "bpm":155,"beats":32,"seed":42
        }))
        .unwrap();
        for context in [false, true] {
            let params = synthesis_params(&request, 16.0, 42, context);
            assert_eq!(params["synth_model"], "acestep-v15-xl-sft-Q8_0.gguf");
            assert_eq!(
                params["task_type"],
                if context { "lego" } else { "text2music" }
            );
            assert_eq!(params["bpm"], 155.0);
            assert_eq!(params["seed"], 42);
            assert_eq!(params["inference_steps"], 80);
            assert_eq!(params["guidance_scale"], 1.0);
        }
        let required: Vec<_> = music_core::manifest::manifest()
            .models
            .into_iter()
            .filter(|m| REQUIRED_MODELS.contains(&m.file.as_str()))
            .collect();
        assert_eq!(required.len(), 3);
        assert!(required.iter().any(|m| m.file == DEFAULT_MODEL));
        assert!(required.iter().all(|m| m.kind != "pro"));
    }

    #[test]
    fn custom_passes_override_defaults_for_text_and_context() {
        for steps in [32, 50, 100] {
            let request: GenerationRequest = serde_json::from_value(json!({
                "slot":36,"instrument":"bass","prompt":"Warm sub", "key":"A minor",
                "bpm":120,"beats":16,"seed":42,"steps":steps
            }))
            .unwrap();
            request.validate().unwrap();
            for context in [false, true] {
                let params = synthesis_params(&request, 10.0, 42, context);
                assert_eq!(params["inference_steps"], steps);
                assert_eq!(params["guidance_scale"], 1.0);
            }
        }
    }
}
