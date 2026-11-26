//! QN Bench — Quantum Networking Optical Bench (Native macOS/Windows + WebAssembly).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod emblem;
mod glossary;
mod levels;
mod model;
mod qkd;
mod render_bench;
mod render_scopes;
mod sim;
mod ui_modals;
#[cfg(not(target_arch = "wasm32"))]
mod updater;
mod version;
mod video;

use app::QnBenchApp;
#[cfg(not(target_arch = "wasm32"))]
use version::{APP_NAME, APP_VERSION};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("{APP_NAME} v{APP_VERSION} — Quantum Networking Optical Bench"))
            .with_inner_size([1160.0, 690.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(QnBenchApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;

    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("no window")
            .document()
            .expect("no document");

        let canvas = document
            .get_element_by_id("qn_bench_canvas")
            .expect("missing #qn_bench_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("element is not HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(QnBenchApp::new(cc)))),
            )
            .await;

        if let Some(loader) = document.get_element_by_id("loading") {
            match start_result {
                Ok(_) => loader.remove(),
                Err(e) => {
                    loader.set_inner_html(&format!(
                        "<div class='loader-box'><p class='loader-title'>WebGL Error</p><p class='loader-sub'>Failed to start QN Bench: {e:?}</p></div>"
                    ));
                }
            }
        }
    });
}
