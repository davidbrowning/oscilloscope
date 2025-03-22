use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{self, Color32};
use egui_plot::{Plot, Line, Legend};
use std::sync::mpsc::{channel, Receiver, Sender};
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
    tx: Sender<f32>,
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
    let (sys_tx, sys_rx) = channel::<f32>();
    let (mic_tx, mic_rx) = channel::<f32>();
    
    let mut sys_stream = build_audio_stream(AudioSource::SystemOutput, sys_tx.clone())?;
    let mut mic_stream = build_audio_stream(AudioSource::Microphone, mic_tx.clone())?;
    
    sys_stream.stream.play()?;
    mic_stream.stream.play()?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Oscilloscope",
        options,
        Box::new(move |_cc| Box::new(MyApp {
            sys_rx,
            mic_rx,
            sys_samples: VecDeque::new(),
            mic_samples: VecDeque::new(),
            sys_stream,
            mic_stream,
            sys_tx,
            mic_tx,
        })),
    )?;

    Ok(())
}

struct MyApp {
    sys_rx: Receiver<f32>,
    mic_rx: Receiver<f32>,
    sys_samples: VecDeque<f32>,
    mic_samples: VecDeque<f32>,
    sys_stream: AudioStream,
    mic_stream: AudioStream,
    sys_tx: Sender<f32>,
    mic_tx: Sender<f32>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set global visuals (this affects the entire app, so we'll refine per-plot styling)
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgba_premultiplied(30, 30, 30, 255); // Dark gray background for the app
        ctx.set_visuals(visuals);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Collect system output samples
            while let Ok(sample) = self.sys_rx.try_recv() {
                if self.sys_samples.len() >= 1000 {
                    self.sys_samples.pop_front();
                }
                self.sys_samples.push_back(sample);
            }

            // Collect microphone samples
            while let Ok(sample) = self.mic_rx.try_recv() {
                if self.mic_samples.len() >= 1000 {
                    self.mic_samples.pop_front();
                }
                self.mic_samples.push_back(sample);
            }

            ui.label("System Output");
            ui.style_mut().visuals.panel_fill = Color32::BLACK; // Black background for system output plot
            Plot::new("System Waveform")
                .height(200.0)
                .allow_zoom(false)
                .allow_drag(false)
                .include_y(-400.0)
                .include_y(400.0)
                .include_x(0.0)
                .include_x(1000.0)
                .legend(Legend::default())
                .show_background(true)
                .show(ui, |plot_ui| {
                    let points: Vec<[f64; 2]> = self.sys_samples.iter()
                        .enumerate()
                        .map(|(i, &sample)| {
                            let y = (sample * 1000.0).clamp(-400.0, 400.0);
                            [i as f64, y as f64]
                        })
                        .collect();
                    plot_ui.line(Line::new(points).color(Color32::from_rgb(0, 191, 255))); // Electric blue
                });

            ui.label("Microphone Input");
            ui.style_mut().visuals.panel_fill = Color32::WHITE; // White background for mic plot
            Plot::new("Mic Waveform")
                .height(200.0)
                .allow_zoom(false)
                .allow_drag(false)
                .include_y(-400.0)
                .include_y(400.0)
                .include_x(0.0)
                .include_x(1000.0)
                .legend(Legend::default())
                .show_background(true)
                .show(ui, |plot_ui| {
                    let points: Vec<[f64; 2]> = self.mic_samples.iter()
                        .enumerate()
                        .map(|(i, &sample)| {
                            let y = (sample * 1000.0).clamp(-400.0, 400.0);
                            [i as f64, y as f64]
                        })
                        .collect();
                    plot_ui.line(Line::new(points).color(Color32::RED)); // Red line
                });
        });
        ctx.request_repaint();
    }
}