use std::{sync::Arc, thread, time::Duration};

use anni_playback::types::PlayerEvent;
use anni_player::AnniPlayer;
use anni_provider::providers::TypedPriorityProvider;

const AUTH: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NTIzNjA5MTUsInR5cGUiOiJ1c2VyIiwidXNlcl9pZCI6InNueWxvbnVlIiwic2hhcmUiOnsia2V5X2lkIjoiYjViNjA1ZDgtOTM0OS00NWY1LWJiZjYtNzU2OTBjYjU4MDkwIiwic2VjcmV0IjoiN2Y3NzVjNmItODEyMy00NGRmLTg4OTctYWUyYTJlM2MwOGVhIn19.-8wPQU7mxIS2xPhPM2TvA9IC20zQkDtQ1P31yA6tQ3U";

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let (player, receiver) = AnniPlayer::new(TypedPriorityProvider::new(vec![]));
    let player = Arc::new(player);

    let handle = thread::Builder::new()
        .name("anni-player".to_owned())
        .spawn({
            let player = Arc::clone(&player);
            move || loop {
                if let Ok(event) = receiver.recv() {
                    log::trace!("received event: {event:#?}");
                    match event {
                        PlayerEvent::Stop => match player.play_next() {
                            Ok(_) => player.play(),
                            Err(e) => log::error!("{e}"),
                        },
                        _ => {}
                    }
                }
            }
        })
        .unwrap();

    player.add_provider(
        "https://annil_serverless.shuttleapp.rs".to_owned(),
        AUTH.to_owned(),
        0,
    );

    player.push_track("c9fc407b-33db-40fb-9e21-550ba3ea5d6b/1/1".parse().unwrap());
    player.push_track("0101753f-e876-46ef-85e7-71f144a5d918/1/4".parse().unwrap());
    player.push_track("0101753f-e876-46ef-85e7-71f144a5d918/1/5".parse().unwrap());
    player.play_next()?;
    thread::sleep(Duration::from_secs(5));
    player.pause();
    thread::sleep(Duration::from_secs(3));
    player.play();
    thread::sleep(Duration::from_secs(1));
    player.play_next()?;

    let _ = handle.join();
    Ok(())
}
