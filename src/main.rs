use std::{io, thread};
use std::error::Error;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::{event, ExecutableCommand, terminal};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{Event, KeyCode};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use rusty_audio::Audio;

use rust_game_space_invaders::{frame, render};
use rust_game_space_invaders::frame::{Drawable, new_frame};
use rust_game_space_invaders::invaders::Invaders;
use rust_game_space_invaders::player::Player;

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = Audio::new();

    audio.add("explode", "explode.wav");
    audio.add("loose", "loose.wav");
    audio.add("move", "move.wav");
    audio.add("pew", "pew.wav");
    audio.add("startup", "startup.wav");
    audio.add("win", "win.wav");

    audio.play("startup");

    // Terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;

    // Render loop in  a separate thread
    let (render_tx, render_rx) = mpsc::channel();
    // better performance and more fatures with crossbeam
    // https://docs.rs/crossbeam-channel/latest/crossbeam_channel/
    let render_handle = thread::spawn(move || {
        let mut last_frame = frame::new_frame();
        let mut stdout = io::stdout();
        render::render(&mut stdout, &last_frame, &last_frame, true);
        loop {
            let curr_frame = match render_rx.recv() {
                Ok(x) => x,
                Err(_) => break,
            };
            render::render(&mut stdout, &last_frame, &curr_frame, false);
            last_frame = curr_frame;
        }
    });

    // Game Loop
    let mut player = Player::new();
    let mut instant = Instant::now();
    let mut invaders = Invaders::new();

    'gameloop: loop {
        // Per-frame init
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut curr_frame = new_frame();
        // Input
        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Left => player.move_left(),
                    KeyCode::Right => player.move_right(),
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        if player.shoot() {
                            audio.play("pew");
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("loose");
                        break 'gameloop;
                    }
                    _ => {}
                }
            }
        }

        // Updates
        player.update(delta);
        if invaders.update(delta) {
            audio.play("move");
        }

        if player.detect_hits(&mut invaders) {
            audio.play("explode");
        }

        // Draw & Render
        let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];
        for drawable in drawables {
            drawable.draw(&mut curr_frame);
        }

        let _ = render_tx.send(curr_frame); // let _ ignores errors suring startup
        thread::sleep(Duration::from_millis(1));

        // win or loose

        if invaders.all_killed() {
            audio.play("win");
            break 'gameloop;
        }

        if invaders.reached_bottom() {
            audio.play("loose");
            break 'gameloop;
        }
    }

    // Cleanup
    drop(render_tx);
    render_handle.join().unwrap();
    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    audio.wait();
    Ok(())
}
