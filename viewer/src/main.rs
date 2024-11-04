use std::net::Ipv4Addr;

use data::PacketInfo;
use eframe::{run_native, App, CreationContext};
use egui::{Context, Pos2};
use egui_file::FileDialog;
use egui_graphs::{
    to_graph, DefaultGraphView, Graph, SettingsInteraction, SettingsNavigation, SettingsStyle,
};
use petgraph::stable_graph::{DefaultIx, NodeIndex, StableGraph};
use petgraph::Directed;
use rand::Rng;
use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

mod drawers;
mod settings;

pub struct SegViewerApp {
    g: Graph,
    packets: Vec<PacketInfo>,
    hosts: HashSet<Ipv4Addr>,
    dirty: bool,
    data_imported: bool,
    opended_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,

    pan: [f32; 2],
    zoom: f32,
}

impl SegViewerApp {
    fn new(_: &CreationContext<'_>) -> Self {
        let mut g = Graph::new(StableGraph::default());
        //let g = generate_random_graph(20, 10);

        g.add_node_with_label((), "init node work around".to_string());

        Self {
            g,
            packets: vec![],
            hosts: HashSet::new(),
            dirty: false,
            data_imported: false,
            opended_file: None,
            open_file_dialog: None,
            pan: [0., 0.],
            zoom: 0.,
        }
    }

    fn update_graph(&mut self) {
        self.dirty = false;
        //let mut g = Graph::new(self.g.g.clone());
        // Create nodes from hosts
        for host in self.hosts.iter() {
            self.g.add_node_with_label((), host.to_string());
        }

        // Create edges between nodes (hosts)
        for packet in self.packets.iter() {}

        //self.g = g;
    }
}

impl App for SegViewerApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        // If we have a file dialog then show it
        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    self.opended_file = Some(file.to_path_buf());
                    println!("{:?}", self.opended_file);
                }
            }
        }

        // If we have a file we need to parse each line to data::PacketInfo instances
        if let Some(open_file_path) = &self.opended_file {
            let file = File::open(open_file_path).unwrap();
            let packet_infos: Vec<PacketInfo> = io::BufReader::new(file)
                .lines()
                .map(|line| {
                    let line = line.unwrap();
                    let packet_info: PacketInfo = serde_json::from_str(&line).unwrap();
                    // push each host into the hosts hashset
                    self.hosts.insert(packet_info.source_ip);
                    self.hosts.insert(packet_info.listener_ip);

                    packet_info
                })
                .collect::<Vec<_>>();
            self.packets = packet_infos;

            println!("{:?}", self.g);

            // Clear the app state
            self.opended_file = None;
            self.open_file_dialog = None;

            self.data_imported = true;
            self.dirty = true;
        }

        // update the underlying graph
        if self.dirty {
            self.update_graph();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Import data").clicked() {
                let filter = Box::new({
                    let ext = Some(OsStr::new("jsonl"));
                    move |path: &Path| -> bool { path.extension() == ext }
                });
                let mut dialog =
                    FileDialog::open_file(self.opended_file.clone()).show_files_filter(filter);
                dialog.open();
                self.open_file_dialog = Some(dialog);
            }
            ui.add_space(10.);

            if ui.button("Add node").clicked() {
                if self.data_imported {
                    self.g.add_node_with_label((), "Foo".to_string());
                }
                println!("{:?}", self.g);
            }

            let interaction_settings = &SettingsInteraction::new()
                .with_dragging_enabled(true)
                .with_node_clicking_enabled(true)
                .with_node_selection_enabled(true)
                .with_node_selection_multi_enabled(true)
                .with_edge_clicking_enabled(true)
                .with_node_selection_enabled(true)
                .with_edge_selection_multi_enabled(true);

            let style_settings = &SettingsStyle::new().with_labels_always(true);
            let navigation_settings = &SettingsNavigation::new()
                .with_zoom_and_pan_enabled(true)
                .with_fit_to_screen_enabled(false);

            ctx.set_zoom_factor(self.zoom);

            ui.add(
                &mut DefaultGraphView::new(&mut self.g)
                    .with_styles(style_settings)
                    .with_interactions(interaction_settings)
                    .with_navigations(navigation_settings),
            );
        });
    }
}

fn generate_random_graph(node_count: usize, edge_count: usize) -> Graph {
    let mut rng = rand::thread_rng();
    let mut graph = StableGraph::new();

    for _ in 0..node_count {
        graph.add_node(());
    }

    for _ in 0..edge_count {
        let source = rng.gen_range(0..node_count);
        let target = rng.gen_range(0..node_count);

        graph.add_edge(NodeIndex::new(source), NodeIndex::new(target), ());
    }

    to_graph(&graph)
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    run_native(
        "Seg Viewer",
        native_options,
        Box::new(|cc| Ok(Box::new(SegViewerApp::new(cc)))),
    )
    .unwrap();
}
