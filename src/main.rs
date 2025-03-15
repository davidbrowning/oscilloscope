use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use egui_plot::{Plot, Line}; // Import from egui_plot
use std::sync::mpsc::{channel, Receiver};
use std::collections::VecDeque;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let input_device = host.default_input_device().expect("No input device available");
    let config: cpal::StreamConfig = input_device.default_input_config()?.into();

    let (tx, rx) = channel::<f32>();
    let stream = input_device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for &sample in data {
                let _ = tx.send(sample);
            }
        },
        |err| eprintln!("Error: {:?}", err),
        None,
    )?;
    stream.play()?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Oscilloscope",
        options,
        Box::new(|_cc| Box::new(MyApp {
            rx,
            samples: VecDeque::new(),
        })),
    )?;

    Ok(())
}

struct MyApp {
    rx: Receiver<f32>,
    samples: VecDeque<f32>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            while let Ok(sample) = self.rx.try_recv() {
                if self.samples.len() >= 500 {
                    self.samples.pop_front();
                }
                self.samples.push_back(sample);
            }

            ui.label("Oscilloscope");
            println!("samples at 1: {} ", self.samples[0]);
            Plot::new("Waveform")
                .height(400.0)
                .data_aspect(1.0)
                .show(ui, |plot_ui| {
                    let points: Vec<[f64; 2]> = self.samples.iter()
                        .enumerate()
                        .map(|(i, &sample)| [i as f64, (sample * 100.0) as f64]) // Amplify by 100
                        .collect();
                    plot_ui.line(Line::new(points));
            });
        });
        ctx.request_repaint();
    }
}
