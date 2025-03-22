use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use egui_plot::{Plot, Line, PlotBounds}; // Keep PlotBounds in imports
use std::sync::mpsc::{channel, Receiver};
use std::collections::VecDeque;

enum AudioSource {
    Microphone,
    SystemOutput,
}

struct AudioStream {
    stream: cpal::Stream,
    source: AudioSource,
}

fn build_audio_stream(
    source: AudioSource,
    tx: std::sync::mpsc::Sender<f32>,
) -> Result<AudioStream, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    
    match source {
        AudioSource::Microphone => {
            let device = host.default_input_device().expect("No input device available");
            let config: cpal::StreamConfig = device.default_input_config()?.into();
            
            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for &sample in data {
                        let _ = tx.send(sample);
                    }
                },
                |err| eprintln!("Error: {:?}", err),
                None,
            )?;
            
            Ok(AudioStream { stream, source })
        }
        AudioSource::SystemOutput => {
            let device = host.default_output_device().expect("No output device available");
            let config: cpal::StreamConfig = device.default_output_config()?.into();
            
            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for &sample in data {
                        let _ = tx.send(sample);
                    }
                },
                |err| eprintln!("Error: {:?}", err),
                None,
            )?;
            
            Ok(AudioStream { stream, source })
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = channel::<f32>();
    let mut audio_stream = build_audio_stream(AudioSource::Microphone, tx.clone())?;
    audio_stream.stream.play()?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Oscilloscope",
        options,
        Box::new(move |_cc| Box::new(MyApp {
            rx,
            samples: VecDeque::new(),
            audio_stream,
            tx,
        })),
    )?;

    Ok(())
}

struct MyApp {
    rx: Receiver<f32>,
    samples: VecDeque<f32>,
    audio_stream: AudioStream,
    tx: std::sync::mpsc::Sender<f32>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let current_source = match self.audio_stream.source {
                    AudioSource::Microphone => "Microphone",
                    AudioSource::SystemOutput => "System Output",
                };
                
                ui.label("Audio Source:");
                if ui.button(current_source).clicked() {
                    let new_source = match self.audio_stream.source {
                        AudioSource::Microphone => AudioSource::SystemOutput,
                        AudioSource::SystemOutput => AudioSource::Microphone,
                    };
                    
                    if let Ok(new_stream) = build_audio_stream(new_source, self.tx.clone()) {
                        self.audio_stream = new_stream;
                        let _ = self.audio_stream.stream.play();
                    }
                }
            });

            while let Ok(sample) = self.rx.try_recv() {
                if self.samples.len() >= 1000 {
                    self.samples.pop_front();
                }
                self.samples.push_back(sample);
            }

            ui.label("Oscilloscope");
            Plot::new("Waveform")
                .height(400.0)
                .allow_zoom(false) // Prevents zooming
                .allow_drag(false) // Prevents panning
                .include_y(-400.0) // Ensure -400 is visible
                .include_y(400.0)  // Ensure 400 is visible
                .include_x(0.0)    // Ensure x starts at 0
                .include_x(1000.0) // Ensure x ends at 1000
                .show(ui, |plot_ui| {
                    let points: Vec<[f64; 2]> = self.samples.iter()
                        .enumerate()
                        .map(|(i, &sample)| {
                            let y = (sample * 1000.0).clamp(-400.0, 400.0); // Clamp values to ±400
                            [i as f64, y as f64]
                        })
                        .collect();
                    plot_ui.line(Line::new(points));
                });
        });
        ctx.request_repaint();
    }
}
