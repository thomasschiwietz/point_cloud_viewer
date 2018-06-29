// Copyright 2016 The Cartographer Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate cgmath;
extern crate clap;
extern crate fnv;
extern crate lru_cache;
extern crate point_viewer;
extern crate point_viewer_grpc;
extern crate rand;
extern crate sdl2;
extern crate time;
extern crate protobuf;

/// Unsafe macro to create a static null-terminated c-string for interop with OpenGL.
#[macro_export]
macro_rules! c_str {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const i8
    }
}

mod glhelper;
mod camera;
#[allow(non_upper_case_globals)]
pub mod opengl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

include!(concat!(env!("OUT_DIR"), "/proto.rs"));

pub mod box_drawer;
pub mod heightmap_drawer;
pub mod graphic;
pub mod node_drawer;

use box_drawer::BoxDrawer;
use camera::Camera;
use cgmath::{Matrix4, Vector3};
use fnv::FnvHashMap;
use node_drawer::{NodeDrawer, NodeViewContainer};
use point_viewer::color::YELLOW;
use point_viewer::color::RED;
use point_viewer::octree::{self, Octree};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Scancode;
use sdl2::video::GLProfile;
use std::cmp;
use std::fs;
use std::error::Error;
use std::sync::Arc;

type OctreeFactory = fn(&String) -> Result<Box<Octree>, Box<Error>>;

pub struct SdlViewer {
    octree_factories: FnvHashMap<String, OctreeFactory>,
}

impl SdlViewer {
    pub fn new() -> Self {
        SdlViewer {
            octree_factories: FnvHashMap::default(),
        }
    }

    // Registers a callback 'function' that is called whenever the octree commandline argument
    // starts with its 'prefix'
    // The callback function creates and returns an Octree
    pub fn register_octree_factory(mut self, prefix: String, function: OctreeFactory) -> SdlViewer {
        self.octree_factories.insert(prefix, function);
        self
    }

    fn load_next_height_map(drawer: &mut heightmap_drawer::HeightMapDrawer, file_name: String, current_index: i32, direction: i32, use_vertex_normals: bool) -> i32 {
        let mut index = current_index;
        let mut loop_count = 0;
        loop {
            index += direction;
            if index < 0 {
                println!("no height map found before index 0");
                index = 0;
                break;
            }
            let file_name = SdlViewer::get_height_map_file_name(&file_name, index);
            if !fs::metadata(file_name).is_err() {
                break;
            }
            loop_count += 1;
            if loop_count > 500 {
                println!("no height map found after index {}", current_index);
                return current_index
            }
        }
        SdlViewer::load_height_map(drawer, file_name, index, use_vertex_normals);
        index
    }

    fn get_height_map_file_name(file_name: &String, index: i32) -> String {
        format!("{}{:06}.pb", file_name, index)
    }

    fn load_height_map(drawer: &mut heightmap_drawer::HeightMapDrawer, file_name: String, index: i32, use_vertex_normals: bool) {
        drawer.load_proto(SdlViewer::get_height_map_file_name(&file_name, index), use_vertex_normals);
    }

    pub fn run(self) {
        let matches = clap::App::new("sdl_viewer")
            .args(&[
                clap::Arg::with_name("octree")
                    .help("Input path of the octree.")
                    .index(1)
                    .required(true),
                clap::Arg::with_name("cache_size_mb")
                    .help(
                        "Maximum cache size in MB for octree nodes in GPU memory. \
                         The default value is 2000 MB and the valid range is 1000 MB to 16000 MB.",
                    )
                    .required(false),
                clap::Arg::with_name("height_map_file_name")
                    .long("height_map_file_name")
                    .help("The file name of a height map protobuf.")
                    .takes_value(true)
                    .required(false),
            ])
            .get_matches();

        let octree_argument = matches.value_of("octree").unwrap();

        // Maximum number of MB for the octree node cache. The default is 2 GB
        let cache_size_mb = matches
            .value_of("cache_size_mb")
            .unwrap_or("2000")
            .parse()
            .unwrap();

        let maybe_height_map_file_name = matches.value_of("height_map_file_name");

        // Maximum number of MB for the octree node cache in range 1..16 GB. The default is 2 GB
        let limit_cache_size_mb = cmp::max(1000, cmp::min(16_000, cache_size_mb));

        // Assuming about 200 KB per octree node on average
        let max_nodes_in_memory = limit_cache_size_mb * 5;

        // call octree generation functions
        let mut octree_opt: Option<Box<Octree>> = None;
        for (prefix, octree_factory_function) in &self.octree_factories {
            if !octree_argument.starts_with(prefix) {
                continue;
            }
            let no_prefix = &octree_argument[prefix.len()..].to_string();
            if let Ok(o) = octree_factory_function(no_prefix) {
                octree_opt = Some(o);
                break;
            }
        }

        // If no octree was generated create an FromDisc loader
        let octree = Arc::new(octree_opt.unwrap_or_else(|| {
            Box::new(octree::OnDiskOctree::new(&octree_argument).unwrap()) as Box<Octree>
        }));

        let ctx = sdl2::init().unwrap();
        let video_subsystem = ctx.video().unwrap();

        let gl_attr = video_subsystem.gl_attr();

        // TODO(hrapp): This should use OpenGL ES 2.0 to be compatible with WebGL, so this can be made
        // to work with emscripten.
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_version(3, 2);

        const WINDOW_WIDTH: i32 = 800 * 2;
        const WINDOW_HEIGHT: i32 = 600 * 2;
        let window = match video_subsystem
            .window("sdl2_viewer", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
            .position_centered()
            .resizable()
            .opengl()
            .build()
        {
            Ok(window) => window,
            Err(err) => panic!("failed to create window: {}", err),
        };

        // We need to create a context now, only after can we actually legally load the gl functions
        // and query 'gl_attr'.
        let _context = window.gl_create_context().unwrap();
        video_subsystem.gl_set_swap_interval(1);

        assert_eq!(gl_attr.context_profile(), GLProfile::Core);

        let gl = opengl::Gl::load_with(|s| {
            let ptr = video_subsystem.gl_get_proc_address(s);
            unsafe { std::mem::transmute(ptr) }
        });

        let node_drawer = NodeDrawer::new(&gl);
        let mut node_views = NodeViewContainer::new(Arc::clone(&octree), max_nodes_in_memory);
        let mut visible_nodes = Vec::new();

        let box_drawer = BoxDrawer::new(&gl);
        let octree_box_color = YELLOW;
        let mut show_octree_nodes = false;

        let mut current_height_index = 0;
        let mut use_vertex_normals = false;
        let mut height_map_drawer = heightmap_drawer::HeightMapDrawer::new(&gl);
        if !maybe_height_map_file_name.is_none() {
            SdlViewer::load_height_map(&mut height_map_drawer, maybe_height_map_file_name.unwrap().to_string(), 0, use_vertex_normals);
        }
        let mut show_points = true;
        let mut show_heightmap = true;
        let mut show_wire_frame = false;

        let mut camera = Camera::new(&gl, WINDOW_WIDTH, WINDOW_HEIGHT);

        let mut events = ctx.event_pump().unwrap();
        let mut num_frames = 0;
        let mut last_log = time::PreciseTime::now();
        let mut force_load_all = false;
        let mut point_size = 2.;
        let mut gamma = 1.5;
        let mut max_level_moving = 8;
        let mut last_moving = time::PreciseTime::now();
        let mut needs_drawing = true;
        let mut last_drawing = last_moving;
        'outer_loop: loop {
            for event in events.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'outer_loop,
                    Event::KeyDown {
                        scancode: Some(code),
                        ..
                    } => {
                        needs_drawing = true;
                        match code {
                            Scancode::Escape => break 'outer_loop,
                            Scancode::W => camera.moving_forward = true,
                            Scancode::S => camera.moving_backward = true,
                            Scancode::A => camera.moving_left = true,
                            Scancode::D => camera.moving_right = true,
                            Scancode::Z => camera.moving_down = true,
                            Scancode::Q => camera.moving_up = true,
                            Scancode::F => force_load_all = true,
                            Scancode::O => show_octree_nodes = !show_octree_nodes,
                            Scancode::Num1 => max_level_moving -= 1,
                            Scancode::Num2 => max_level_moving += 1,
                            Scancode::Num3 => { current_height_index = SdlViewer::load_next_height_map(&mut height_map_drawer, maybe_height_map_file_name.unwrap().to_string(), current_height_index, -1, use_vertex_normals); },
                            Scancode::Num4 => { current_height_index = SdlViewer::load_next_height_map(&mut height_map_drawer, maybe_height_map_file_name.unwrap().to_string(), current_height_index,  1, use_vertex_normals); },
                            Scancode::Num5 => { show_points = !show_points; needs_drawing = true; },
                            Scancode::Num6 => { show_heightmap = !show_heightmap; needs_drawing = true; },
                            Scancode::Y => { show_wire_frame = !show_wire_frame; needs_drawing = true; },
                            Scancode::T => { use_vertex_normals = !use_vertex_normals; SdlViewer::load_height_map(&mut height_map_drawer, maybe_height_map_file_name.unwrap().to_string(), current_height_index, use_vertex_normals); }
                            Scancode::Num7 => gamma -= 0.1,
                            Scancode::Num8 => gamma += 0.1,
                            Scancode::Num9 => point_size -= 0.1,
                            Scancode::Num0 => point_size += 0.1,
                            _ => (),
                        }
                    }
                    Event::KeyUp {
                        scancode: Some(code),
                        ..
                    } => match code {
                        Scancode::W => camera.moving_forward = false,
                        Scancode::S => camera.moving_backward = false,
                        Scancode::A => camera.moving_left = false,
                        Scancode::D => camera.moving_right = false,
                        Scancode::Z => camera.moving_down = false,
                        Scancode::Q => camera.moving_up = false,
                        _ => (),
                    },
                    Event::MouseMotion {
                        xrel,
                        yrel,
                        mousestate,
                        ..
                    } if mousestate.left() =>
                    {
                        camera.mouse_drag(xrel, yrel)
                    }
                    Event::MouseWheel { y, .. } => {
                        camera.mouse_wheel(y);
                    }
                    Event::Window {
                        win_event: WindowEvent::SizeChanged(w, h),
                        ..
                    } => {
                        camera.set_size(&gl, w, h);
                    }
                    _ => (),
                }
            }

            if camera.update() {
                last_moving = time::PreciseTime::now();
                needs_drawing = true;
                node_drawer.update_world_to_gl(&camera.get_world_to_gl());
                visible_nodes = octree.get_visible_nodes(
                    &camera.get_world_to_gl(),
                    camera.width,
                    camera.height,
                );
            }

            if force_load_all {
                println!("Force loading all currently visible nodes.");
                let visible_node_ids: Vec<_> = visible_nodes.iter().map(|n| n.id).collect();
                node_views.request_all(&visible_node_ids);
                force_load_all = false;
            }

            let mut num_points_drawn = 0;
            let mut num_nodes_drawn = 0;

            let now = time::PreciseTime::now();
            let moving = last_moving.to(now) < time::Duration::milliseconds(150);
            needs_drawing =
                needs_drawing || last_drawing.to(now) < time::Duration::milliseconds(1000);
            if needs_drawing {
                unsafe {
                    gl.ClearColor(0., 0., 0., 1.);
                    gl.Clear(opengl::COLOR_BUFFER_BIT | opengl::DEPTH_BUFFER_BIT);
                }
            }

            // Bisect the actual level to choose, we want to be as close as possible to the max
            // nodes to use.
            let mut max_level_to_display = if moving { max_level_moving } else { 256 };
            let mut min_level_to_display = 0;
            let mut filtered_visible_nodes: Vec<_>;
            while (max_level_to_display - min_level_to_display) > 1 {
                let current = (max_level_to_display + min_level_to_display) / 2;
                filtered_visible_nodes = visible_nodes
                    .iter()
                    .filter(|n| n.id.level() <= current)
                    .collect();
                if filtered_visible_nodes.len() > max_nodes_in_memory {
                    max_level_to_display = current;
                } else {
                    min_level_to_display = current;
                }
            }
            filtered_visible_nodes = visible_nodes
                .iter()
                .filter(|n| n.id.level() <= min_level_to_display)
                .collect();
            assert!(filtered_visible_nodes.len() < max_nodes_in_memory);

            if show_points {
                for visible_node in filtered_visible_nodes {
                    let view = node_views.get_or_request(&visible_node.id, &node_drawer.program);
                    if !needs_drawing || view.is_none() {
                        continue;
                    }
                    let view = view.unwrap();
                    num_points_drawn +=
                        node_drawer.draw(view, 1 /* level of detail */, point_size, gamma);
                    num_nodes_drawn += 1;

                    // debug drawer
                    if show_octree_nodes {
                        box_drawer.draw_outlines(
                            &view.meta.bounding_cube,
                            &camera.get_world_to_gl(),
                            &octree_box_color,
                        );
                    }
                }
            }

            if show_heightmap {
                let color2 = vec![1.,1.,0.,1.];
                height_map_drawer.draw(&color2, &camera.get_world_to_camera(), &camera.get_world_to_gl(), show_wire_frame);
                let mx = camera.get_world_to_gl() * 
                    Matrix4::from_translation(Vector3::new(height_map_drawer.origin.x + height_map_drawer.edge_length * 0.5, height_map_drawer.origin.y + height_map_drawer.edge_length * 0.5, height_map_drawer.origin.z)) *
                    Matrix4::from_nonuniform_scale(height_map_drawer.edge_length * 0.5, height_map_drawer.edge_length * 0.5, 0.);
                box_drawer.draw_outlines_from_transformation(&mx, &octree_box_color);

                let points = [ 
                    Vector3::new(3616.0, 2199.24, -19.3947),
                    Vector3::new(3616.0, 2199.24, -13.128),
                    // Vector3::new(22.847, 117.137, 0.779959), 
                    // Vector3::new(22.847, 117.137, 2.09595)
                    ];
                let red = RED;
                for p in points.into_iter() {
                    let mx = camera.get_world_to_gl() * 
                        Matrix4::from_translation(*p) * 
                        Matrix4::from_scale(100.);
                    box_drawer.draw_outlines_from_transformation(&mx, &red);                    
                }
            }

            if needs_drawing {
                window.gl_swap_window();
                last_drawing = time::PreciseTime::now();
            }
            needs_drawing = moving;

            num_frames += 1;
            let now = time::PreciseTime::now();
            if last_log.to(now) > time::Duration::seconds(1) {
                let duration = last_log.to(now).num_microseconds().unwrap();
                let fps = (num_frames * 1_000_000u32) as f32 / duration as f32;
                num_frames = 0;
                last_log = now;
                println!(
                    "FPS: {:#?}, Drew {} points from {} loaded nodes. {} nodes \
                     should be shown, Cache {} MB",
                    fps,
                    num_points_drawn,
                    num_nodes_drawn,
                    visible_nodes.len(),
                    node_views.get_used_memory_bytes() as f32 / 1024. / 1024.,
                );
            }
        }
    }
}
