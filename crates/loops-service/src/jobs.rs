use super::*;
use music_core::{
    audio::{Audio, RATE},
    loops::{EditRecipe, GenerationRequest, BASE_MODEL},
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
    tokio::spawn(async move {
        let outcome=async {
            let _permit=tokio::select!{_=cancel.cancelled()=>return Err(music_core::error::Error::Cancelled),p=service.serial.acquire()=>p.map_err(|e|message(e.to_string()))?};
            update(&service,&jid,json!({"id":jid,"status":"running","message":"Checking models"})).await;
            let root=service.models.lock().await.clone();let mut verifier=music_core::verification::ModelVerifier::open(&service.root);let client=reqwest::Client::new();let manifest=music_core::manifest::manifest();
            for model in manifest.models.into_iter().filter(|m|m.kind=="pro"||["vae-BF16.gguf","Qwen3-Embedding-0.6B-Q8_0.gguf"].contains(&m.file.as_str())) {
                music_core::download::download_model(&client,&format!("{}/{}",manifest.base_url,model.file),&root,&model,&mut verifier,&cancel,|p|{
                    if let Ok(mut jobs)=service.jobs.try_lock(){if let Some(j)=jobs.get_mut(&jid){if !j.cancel.is_cancelled(){j.value=json!({"id":jid,"status":"running","message":format!("{} {}: {} / {} MiB",p.phase,p.file,p.downloaded/1048576,p.total/1048576)});}}}
                }).await?;
            }
            Ok::<_,music_core::error::Error>(json!({"id":jid,"status":"done","message":format!("Models ready · {}",root.display()),"results":[]}))
        }.await;
        finish(&service, &jid, outcome).await;
    });
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
    tokio::spawn(async move {
        let outcome=async {
            let _permit=tokio::select!{_=cancel.cancelled()=>return Err(music_core::error::Error::Cancelled),p=service.serial.acquire()=>p.map_err(|e|message(e.to_string()))?};
            update(&service,&jid,json!({"id":jid,"status":"running","message":"Starting ACE-Step"})).await;
            let mut guard=service.engine.lock().await;
            if guard.as_mut().is_some_and(|e|!e.alive()){guard.take();}
            if guard.is_none(){*guard=Some(engine::Engine::start(&service,&cancel).await?);}
            let engine=guard.as_ref().unwrap();
            let duration=request.seconds();let generated_duration=duration*(10.0/duration).ceil().max(1.0);
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
                let params=json!({"task_type":if source.is_some(){"lego"}else{"text2music"},"track":request.instrument,"caption":format!("Isolated {} loop. {}",request.instrument,request.prompt),"lyrics":"[Instrumental]","bpm":request.bpm,"duration":generated_duration,"keyscale":request.key,"timesignature":request.numerator.to_string(),"seed":seed,"inference_steps":request.steps,"guidance_scale":1.0,"shift":1.0,"output_format":"wav32","synth_model":BASE_MODEL});
                let bytes=engine.client.generate_stem(&params,source.as_deref(),&cancel,|text|{
                    if let Ok(mut jobs)=service.jobs.try_lock(){if let Some(j)=jobs.get_mut(&jid){if !j.cancel.is_cancelled(){j.value["message"]=json!(format!("Variant {} / {} · {text}",variant+1,request.variants));}}}
                }).await?;
                let audio=Audio::decode(&bytes)?;let target_frames=(duration*RATE as f64).round() as u64;
                if (audio.samples.len() as u64)<target_frames{return Err(message("Engine returned too little audio; existing loop preserved"));}
                let dir=service.root.join("assets");std::fs::create_dir_all(&dir)?;let original=dir.join(format!("{}-original.wav",id()));audio.write(&original)?;
                let edited=music_core::loops::edit(&audio,&EditRecipe{end:target_frames,frames:target_frames,crossfade_ms:5.0,..Default::default()})?;
                let path=dir.join(format!("{}.wav",id()));edited.write(&path)?;
                results.push(json!({"path":path,"original":original,"frames":target_frames,"bpm":request.bpm,"beats":request.beats,"key":request.key,"name":request.instrument,"seed":seed,"recipe":{"start":0,"end":target_frames,"frames":target_frames,"crossfade_ms":5.0}}));
            }
            Ok(json!({"id":jid,"status":"done","message":"Ready to audition — choose a variant","results":results}))
        }.await;
        finish(&service, &jid, outcome).await;
        let _ = std::fs::remove_dir_all(snapshot);
    });
    Ok(result)
}
async fn finish(service: &Service, jid: &str, outcome: Result<Value>) {
    let value = match outcome {
        Ok(v) => v,
        Err(e) => json!({"id":jid,"status":"error","message":e.to_string(),"results":[]}),
    };
    update(service, jid, value).await;
}
