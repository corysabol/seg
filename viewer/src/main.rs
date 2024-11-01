use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::time::Instant;

use crossbeam::channel::{unbounded, Receiver, Sender};
use data::*;
use drawers::ValuesSectionDebug;
use eframe::{run_native, App, CreationContext};
use egui::{CollapsingHeader, Context, ScrollArea, Ui};
use egui_file::FileDialog;
use egui_graphs::events::Event;
use egui_graphs::Graph;
use egui_graphs::GraphView;
use petgraph::stable_graph::{DefaultIx, EdgeIndex, NodeIndex, StableGraph};
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

const EVENTS_LIMIT: usize = 100;

pub struct SegViewerApp {
    g: Graph<(), (), Directed, DefaultIx>,
    settings_graph: settings::SettingsGraph,
    settings_interaction: settings::SettingsInteraction,
    settings_navigation: settings::SettingsNavigation,
    settings_style: settings::SettingsStyle,
    last_events: Vec<String>,
    fps: f32,
    last_update_time: Instant,
    frames_last_time_span: usize,
    event_publisher: Sender<Event>,
    event_consumer: Receiver<Event>,
    pan: [f32; 2],
    zoom: f32,
    packets: Vec<PacketInfo>,
    hosts: HashSet<Ipv4Addr>,
    opended_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
}

impl SegViewerApp {
    fn new(_: &CreationContext<'_>) -> Self {
        let settings_graph = settings::SettingsGraph::default();

        let g = petgraph::stable_graph::StableGraph::new();
        let g = Graph::new(g);

        let (event_publisher, event_consumer) = unbounded();

        Self {
            g,
            event_consumer,
            event_publisher,
            settings_graph,
            settings_interaction: settings::SettingsInteraction::default(),
            settings_navigation: settings::SettingsNavigation::default(),
            settings_style: settings::SettingsStyle::default(),
            last_events: Vec::default(),
            fps: 0.,
            last_update_time: Instant::now(),
            frames_last_time_span: 0,
            pan: [0., 0.],
            zoom: 0.,
            packets: vec![],
            hosts: HashSet::new(),
            opended_file: None,
            open_file_dialog: None,
        }
    }

    fn update_fps(&mut self) {
        self.frames_last_time_span += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update_time);
        if elapsed.as_secs() >= 1 {
            self.last_update_time = now;
            self.fps = self.frames_last_time_span as f32 / elapsed.as_secs_f32();
            self.frames_last_time_span = 0;
        }
    }

    fn handle_events(&mut self) {
        self.event_consumer.try_iter().for_each(|e| {
            if self.last_events.len() > EVENTS_LIMIT {
                self.last_events.remove(0);
            }
            self.last_events.push(serde_json::to_string(&e).unwrap());

            match e {
                Event::Pan(payload) => self.pan = payload.new_pan,
                Event::Zoom(payload) => self.zoom = payload.new_zoom,
                Event::NodeMove(payload) => {
                    let node_id: NodeIndex<usize> = NodeIndex::new(payload.id);
                }
                _ => {}
            }
        });
    }

    fn random_node_idx(&self) -> Option<NodeIndex> {
        let nodes_cnt = self.g.node_count();
        if nodes_cnt == 0 {
            return None;
        }

        let random_n_idx = rand::thread_rng().gen_range(0..nodes_cnt);
        self.g.g.node_indices().nth(random_n_idx)
    }

    fn random_edge_idx(&self) -> Option<EdgeIndex> {
        let edges_cnt = self.g.edge_count();
        if edges_cnt == 0 {
            return None;
        }

        let random_e_idx = rand::thread_rng().gen_range(0..edges_cnt);
        self.g.g.edge_indices().nth(random_e_idx)
    }

    fn remove_random_node(&mut self) {
        let idx = self.random_node_idx().unwrap();
        self.remove_node(idx);
    }

    fn add_random_node(&mut self) {
        let random_n_idx = self.random_node_idx();
        if random_n_idx.is_none() {
            return;
        }

        self.g.node(random_n_idx.unwrap()).unwrap();
    }

    fn remove_node(&mut self, idx: NodeIndex) {
        self.g.remove_node(idx);

        // update edges count
        self.settings_graph.count_edge = self.g.edge_count();
    }

    fn add_random_edge(&mut self) {
        let random_start = self.random_node_idx().unwrap();
        let random_end = self.random_node_idx().unwrap();

        self.add_edge(random_start, random_end);
    }

    fn add_edge(&mut self, start: NodeIndex, end: NodeIndex) {
        self.g.add_edge(start, end, ());
    }

    fn remove_random_edge(&mut self) {
        let random_e_idx = self.random_edge_idx();
        if random_e_idx.is_none() {
            return;
        }
        let endpoints = self.g.edge_endpoints(random_e_idx.unwrap()).unwrap();

        self.remove_edge(endpoints.0, endpoints.1);
    }

    fn remove_edge(&mut self, start: NodeIndex, end: NodeIndex) {
        let (g_idx, _) = self.g.edges_connecting(start, end).next().unwrap();
        self.g.remove_edge(g_idx);
    }

    fn draw_section_widget(&mut self, ui: &mut Ui) {
        CollapsingHeader::new("Navigation")
            .default_open(true)
            .show(ui, |ui| {
                if ui
                    .checkbox(
                        &mut self.settings_navigation.fit_to_screen_enabled,
                        "fit_to_screen",
                    )
                    .changed()
                    && self.settings_navigation.fit_to_screen_enabled
                {
                    self.settings_navigation.zoom_and_pan_enabled = false
                };
                ui.label("Enable fit to screen to fit the graph to the screen on every frame.");

                ui.add_space(5.);

                ui.add_enabled_ui(!self.settings_navigation.fit_to_screen_enabled, |ui| {
                    ui.vertical(|ui| {
                        ui.checkbox(
                            &mut self.settings_navigation.zoom_and_pan_enabled,
                            "zoom_and_pan",
                        );
                        ui.label("Zoom with ctrl + mouse wheel, pan with middle mouse drag.");
                    })
                    .response
                    .on_disabled_hover_text("disable fit_to_screen to enable zoom_and_pan");
                });
            });

        CollapsingHeader::new("Style").show(ui, |ui| {
            ui.checkbox(&mut self.settings_style.labels_always, "labels_always");
            ui.label("Wheter to show labels always or when interacted only.");
        });

        CollapsingHeader::new("Interaction").show(ui, |ui| {
                if ui.checkbox(&mut self.settings_interaction.dragging_enabled, "dragging_enabled").clicked() && self.settings_interaction.dragging_enabled {
                    self.settings_interaction.node_clicking_enabled = true;
                };
                ui.label("To drag use LMB click + drag on a node.");

                ui.add_space(5.);

                ui.add_enabled_ui(!(self.settings_interaction.dragging_enabled || self.settings_interaction.node_selection_enabled || self.settings_interaction.node_selection_multi_enabled), |ui| {
                    ui.vertical(|ui| {
                        ui.checkbox(&mut self.settings_interaction.node_clicking_enabled, "node_clicking_enabled");
                        ui.label("Check click events in last events");
                    }).response.on_disabled_hover_text("node click is enabled when any of the interaction is also enabled");
                });

                ui.add_space(5.);

                ui.add_enabled_ui(!self.settings_interaction.node_selection_multi_enabled, |ui| {
                    ui.vertical(|ui| {
                        if ui.checkbox(&mut self.settings_interaction.node_selection_enabled, "node_selection_enabled").clicked() && self.settings_interaction.node_selection_enabled {
                            self.settings_interaction.node_clicking_enabled = true;
                        };
                        ui.label("Enable select to select nodes with LMB click. If node is selected clicking on it again will deselect it.");
                    }).response.on_disabled_hover_text("node_selection_multi_enabled enables select");
                });

                if ui.checkbox(&mut self.settings_interaction.node_selection_multi_enabled, "node_selection_multi_enabled").changed() && self.settings_interaction.node_selection_multi_enabled {
                    self.settings_interaction.node_clicking_enabled = true;
                    self.settings_interaction.node_selection_enabled = true;
                }
                ui.label("Enable multiselect to select multiple nodes.");

                ui.add_space(5.);

                ui.add_enabled_ui(!(self.settings_interaction.edge_selection_enabled || self.settings_interaction.edge_selection_multi_enabled), |ui| {
                    ui.vertical(|ui| {
                        ui.checkbox(&mut self.settings_interaction.edge_clicking_enabled, "edge_clicking_enabled");
                        ui.label("Check click events in last events");
                    }).response.on_disabled_hover_text("edge click is enabled when any of the interaction is also enabled");
                });

                ui.add_space(5.);

                ui.add_enabled_ui(!self.settings_interaction.edge_selection_multi_enabled, |ui| {
                    ui.vertical(|ui| {
                        if ui.checkbox(&mut self.settings_interaction.edge_selection_enabled, "edge_selection_enabled").clicked() && self.settings_interaction.edge_selection_enabled {
                            self.settings_interaction.edge_clicking_enabled = true;
                        };
                        ui.label("Enable select to select edges with LMB click. If edge is selected clicking on it again will deselect it.");
                    }).response.on_disabled_hover_text("edge_selection_multi_enabled enables select");
                });

                if ui.checkbox(&mut self.settings_interaction.edge_selection_multi_enabled, "edge_selection_multi_enabled").changed() && self.settings_interaction.edge_selection_multi_enabled {
                    self.settings_interaction.edge_clicking_enabled = true;
                    self.settings_interaction.edge_selection_enabled = true;
                }
                ui.label("Enable multiselect to select multiple edges.");
            });

        CollapsingHeader::new("Selected")
            .default_open(true)
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height(200.)
                    .show(ui, |ui| {
                        self.g.selected_nodes().iter().for_each(|node| {
                            ui.label(format!("{node:?}"));
                        });
                        self.g.selected_edges().iter().for_each(|edge| {
                            ui.label(format!("{edge:?}"));
                        });
                    });
            });

        CollapsingHeader::new("Last Events")
            .default_open(true)
            .show(ui, |ui| {
                if ui.button("clear").clicked() {
                    self.last_events.clear();
                }
                ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        self.last_events.iter().rev().for_each(|event| {
                            ui.label(event);
                        });
                    });
            });
    }

    fn draw_section_debug(&self, ui: &mut Ui) {
        drawers::draw_section_debug(
            ui,
            ValuesSectionDebug {
                zoom: self.zoom,
                pan: self.pan,
                fps: self.fps,
            },
        );
    }

    fn reset(&mut self) {
        let settings_graph = settings::SettingsGraph::default();

        let g = Graph::new(petgraph::stable_graph::StableGraph::new());

        self.settings_graph = settings_graph;

        self.g = g;
    }
}

impl App for SegViewerApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        egui::SidePanel::right("right_panel")
            .min_width(250.)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(10.);

                    if ui.button("Import data").clicked() {
                        let filter = Box::new({
                            let ext = Some(OsStr::new("jsonl"));
                            move |path: &Path| -> bool { path.extension() == ext }
                        });
                        let mut dialog = FileDialog::open_file(self.opended_file.clone())
                            .show_files_filter(filter);
                        dialog.open();
                        self.open_file_dialog = Some(dialog);
                    }

                    ui.add_space(10.);

                    //egui::CollapsingHeader::new("Debug")
                    //    .default_open(true)
                    //    .show(ui, |ui| self.draw_section_debug(ui));

                    //ui.add_space(10.);

                    CollapsingHeader::new("Options")
                        .default_open(true)
                        .show(ui, |ui| self.draw_section_widget(ui));
                });
            });

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

            // TODO: construct graph nodes from hosts
            // The nodes should have a label with the IP address and the network tag
            // thought: maybe clicking on a node shows a panel of node info

            // TODO: construct edges from packets
            // The edges should be directed, and have a label with the port and optionally the TCP
            // flags

            // Clear the app state
            self.opended_file = None;
            self.open_file_dialog = None;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let settings_interaction = &egui_graphs::SettingsInteraction::new()
                .with_node_selection_enabled(self.settings_interaction.node_selection_enabled)
                .with_node_selection_multi_enabled(
                    self.settings_interaction.node_selection_multi_enabled,
                )
                .with_dragging_enabled(self.settings_interaction.dragging_enabled)
                .with_node_clicking_enabled(self.settings_interaction.node_clicking_enabled)
                .with_edge_clicking_enabled(self.settings_interaction.edge_clicking_enabled)
                .with_edge_selection_enabled(self.settings_interaction.edge_selection_enabled)
                .with_edge_selection_multi_enabled(
                    self.settings_interaction.edge_selection_multi_enabled,
                );
            let settings_navigation = &egui_graphs::SettingsNavigation::new()
                .with_zoom_and_pan_enabled(self.settings_navigation.zoom_and_pan_enabled)
                .with_fit_to_screen_enabled(self.settings_navigation.fit_to_screen_enabled)
                .with_zoom_speed(self.settings_navigation.zoom_speed);
            let settings_style = &egui_graphs::SettingsStyle::new()
                .with_labels_always(self.settings_style.labels_always);
            ui.add(
                &mut GraphView::new(&mut self.g)
                    .with_interactions(settings_interaction)
                    .with_navigations(settings_navigation)
                    .with_styles(settings_style)
                    .with_events(&self.event_publisher),
            );
        });

        self.handle_events();
        self.update_fps();
    }
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
