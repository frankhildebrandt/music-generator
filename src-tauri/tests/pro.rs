use music_generator::{
    db::Database,
    pro::{
        audio::{mix, Audio, MixSource},
        model::Project,
    },
};

#[test]
fn project_edits_survive_reopening_and_can_be_undone() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let mut project = db.pro_create("Session", 120, "A minor", 768000).unwrap();
    project.name = "Renamed".into();
    let saved = db.pro_save(&project).unwrap();
    assert_eq!(saved.revision, 1);
    assert!(
        db.pro_save(&project).is_err(),
        "stale edits must not overwrite autosave"
    );
    drop(db);
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.pro_project(&saved.id).unwrap().name, "Renamed");
    assert_eq!(db.pro_history(&saved.id, false).unwrap().name, "Session");
    assert_eq!(db.pro_history(&saved.id, true).unwrap().name, "Renamed");
}

#[test]
fn mixer_places_trimmed_audio_at_its_timeline_position() {
    let source = Audio {
        samples: vec![[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]],
    };
    let mixed = mix(
        5,
        &[MixSource {
            audio: &source,
            start: 2,
            offset: 1,
            length: 2,
            gain: 0.5,
            pan: 0.0,
        }],
    );
    assert_eq!(
        mixed.samples,
        vec![[0.0, 0.0], [0.0, 0.0], [0.15, 0.2], [0.25, 0.3], [0.0, 0.0]]
    );
}

#[test]
fn project_rejects_clips_outside_the_canvas() {
    let mut project = Project::new("Session", 120, "A minor", 768000).unwrap();
    project.duration = u64::MAX;
    assert!(project.validate().is_err());
}

#[test]
fn context_excludes_target_even_when_other_tracks_are_muted_or_not_soloed() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let source = dir.path().join("fixture.wav");
    Audio {
        samples: vec![[0.25, 0.5]; 8],
    }
    .write(&source)
    .unwrap();
    let project = db.pro_create("Context", 120, "A minor", 768000).unwrap();
    let project = db.pro_import(&project.id, &source, None).unwrap();
    let mut project = db.pro_import(&project.id, &source, None).unwrap();
    project.tracks[0].gain = 1.0;
    project.tracks[0].solo = true;
    project.tracks[1].gain = 0.5;
    project.tracks[1].mute = true;
    let mixed = music_generator::pro::render::render(
        &db,
        &project,
        true,
        Some(&project.tracks[0].id),
        None,
    )
    .unwrap();
    assert_eq!(mixed.samples[0], [0.125, 0.25]);
    project.tracks[1].context = false;
    assert_eq!(
        music_generator::pro::render::render(
            &db,
            &project,
            true,
            Some(&project.tracks[0].id),
            None
        )
        .unwrap()
        .peak(),
        0.0
    );
}

#[test]
fn jobs_freeze_context_and_late_results_cannot_overwrite_cancellation() {
    use music_generator::pro::model::{Generation, Take};
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let source = dir.path().join("fixture.wav");
    Audio {
        samples: vec![[0.2, 0.2]; 8],
    }
    .write(&source)
    .unwrap();
    let project = db.pro_create("Snapshot", 120, "A minor", 768000).unwrap();
    let mut project = db.pro_import(&project.id, &source, None).unwrap();
    let request = Generation {
        track_id: project.tracks[0].id.clone(),
        clip_id: None,
        prompt: "bass".into(),
        seed: 42,
        steps: 50,
        variants: 1,
        start: 0,
        end: 768000,
    };
    let ids = db.pro_enqueue(&project.id, &request).unwrap();
    project.tracks[0].gain = 0.1;
    db.pro_save(&project).unwrap();
    let (jid, snapshot, _) = db.pro_claim().unwrap().unwrap();
    assert_eq!(snapshot.tracks[0].gain, 0.7);
    db.pro_job_status(&ids[0], "cancelled", "Cancelled")
        .unwrap();
    let take = Take::new(
        &request.track_id,
        "Late",
        8,
        serde_json::json!({}),
        snapshot.revision,
    );
    assert!(!db.pro_complete(&jid, &project.id, &take).unwrap());
    assert_eq!(db.pro_jobs(&project.id).unwrap()[0].status, "cancelled");
    assert_eq!(db.pro_project(&project.id).unwrap().takes.len(), 1);
}

#[test]
fn range_regeneration_preserves_samples_outside_the_selected_source_window() {
    use music_generator::pro::{model::Clip, render::splice};
    let original = Audio {
        samples: vec![[0.1, 0.2]; 20],
    };
    let generated = Audio {
        samples: vec![[0.6, 0.8]; 20],
    };
    let clip = Clip {
        id: "c".into(),
        track_id: "t".into(),
        take_id: "a".into(),
        start: 4,
        offset: 2,
        length: 10,
        gain: 1.0,
    };
    let output = splice(&original, &generated, &clip, 7, 11).unwrap();
    assert_eq!(&output.samples[..5], &original.samples[..5]);
    assert_eq!(&output.samples[9..], &original.samples[9..]);
    assert_ne!(output.samples[6], original.samples[6]);
}

#[test]
fn moving_the_library_keeps_project_takes_readable() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let source = dir.path().join("fixture.wav");
    Audio {
        samples: vec![[0.2, 0.3]; 8],
    }
    .write(&source)
    .unwrap();
    let project = db.pro_create("Move", 120, "A minor", 768000).unwrap();
    let project = db.pro_import(&project.id, &source, None).unwrap();
    let destination = dir.path().join("moved songs");
    db.change_songs_dir(&destination).unwrap();
    let path = db.pro_path(&project.id, &project.takes[0].file).unwrap();
    assert_eq!(Audio::read(&path).unwrap().samples[0], [0.2, 0.3]);
}

#[test]
fn importing_a_desktop_profile_preserves_pro_projects_and_audio() {
    let old = tempfile::tempdir().unwrap();
    let source = Database::open(old.path()).unwrap();
    let wav = old.path().join("fixture.wav");
    Audio {
        samples: vec![[0.25, 0.5]; 8],
    }
    .write(&wav)
    .unwrap();
    let project = source.pro_create("Import", 120, "A minor", 768000).unwrap();
    let project = source.pro_import(&project.id, &wav, None).unwrap();
    let new = tempfile::tempdir().unwrap();
    let db = Database::open(new.path()).unwrap();
    music_generator::import::import_legacy(&db, old.path()).unwrap();
    let copied = db.pro_project(&project.id).unwrap();
    assert_eq!(copied.name, "Import");
    assert_eq!(
        Audio::read(&db.pro_path(&copied.id, &copied.takes[0].file).unwrap())
            .unwrap()
            .samples[0],
        [0.25, 0.5]
    );
    assert!(source
        .pro_path(&project.id, &project.takes[0].file)
        .unwrap()
        .is_file());
}

#[test]
fn restarting_resumes_only_interrupted_jobs_and_keeps_cancelled_jobs_cancelled() {
    use music_generator::pro::model::Generation;
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let wav = dir.path().join("fixture.wav");
    Audio {
        samples: vec![[0.1, 0.1]; 8],
    }
    .write(&wav)
    .unwrap();
    let project = db.pro_create("Resume", 120, "A minor", 768000).unwrap();
    let project = db.pro_import(&project.id, &wav, None).unwrap();
    let request = Generation {
        track_id: project.tracks[0].id.clone(),
        clip_id: None,
        prompt: "bass".into(),
        seed: 10,
        steps: 50,
        variants: 2,
        start: 0,
        end: 768000,
    };
    let ids = db.pro_enqueue(&project.id, &request).unwrap();
    db.pro_claim().unwrap();
    db.pro_job_status(&ids[1], "cancelled", "Cancelled")
        .unwrap();
    drop(db);
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.pro_claim().unwrap().unwrap().0, ids[0]);
    assert_eq!(
        db.pro_jobs(&project.id)
            .unwrap()
            .iter()
            .find(|j| j.id == ids[1])
            .unwrap()
            .status,
        "cancelled"
    );
}

#[test]
fn stem_export_aligns_all_tracks_and_mix_export_respects_mute() {
    use music_generator::pro::render::export;
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let wav = dir.path().join("fixture.wav");
    Audio {
        samples: vec![[0.25, 0.5]; 8],
    }
    .write(&wav)
    .unwrap();
    let project = db.pro_create("Export", 120, "A minor", 48000).unwrap();
    let mut project = db.pro_import(&project.id, &wav, None).unwrap();
    project.tracks[0].mute = true;
    let mix = dir.path().join("mix.wav");
    export(&db, &project, &mix, false).unwrap();
    assert_eq!(Audio::read(&mix).unwrap().peak(), 0.0);
    let zip = dir.path().join("stems.zip");
    export(&db, &project, &zip, true).unwrap();
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip).unwrap()).unwrap();
    let mut stem = archive.by_index(0).unwrap();
    let mut bytes = vec![];
    std::io::Read::read_to_end(&mut stem, &mut bytes).unwrap();
    let audio = Audio::decode(&bytes).unwrap();
    assert_eq!(audio.samples.len(), 48000);
    assert!(audio.peak() > 0.1);
}
