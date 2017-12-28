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
extern crate point_viewer;
extern crate rand;
extern crate sdl2;
extern crate time;
#[macro_use]
extern crate sdl_viewer;
extern crate clap;

use cgmath::{Array, Matrix, Matrix4, Vector3, Perspective};
use cgmath::{Angle, Decomposed, Deg, InnerSpace, One, Quaternion, Rad, Rotation,
             Rotation3, Transform, Zero};
use point_viewer::math::{Cuboid, CuboidLike, Frustum, Vector4f, Vector3f, Matrix4f};
use point_viewer::octree;
use rand::{Rng, thread_rng};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Scancode;
use sdl2::video::GLProfile;
use sdl_viewer::{Camera, gl};
use sdl_viewer::boxdrawer::OutlinedBoxDrawer;
use sdl_viewer::quad_drawer::QuadDrawer;
use sdl_viewer::gl::types::{GLboolean, GLint, GLsizeiptr, GLuint};
use sdl_viewer::graphic::{GlBuffer, GlProgram, GlVertexArray, GlQuery, GlFramebuffer};
use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::mem;
use std::path::PathBuf;
use std::process;
use std::ptr;
use std::str;

const FRAGMENT_SHADER_POINTS: &'static str = include_str!("../shaders/points.fs");
const VERTEX_SHADER_POINTS: &'static str = include_str!("../shaders/points.vs");

fn get_bounding_box(min: &Vector3f, max: &Vector3f, matrix: &Matrix4f) -> Cuboid {
    let mut rv = Cuboid::new();
    for p in &[
        Vector4f::new(min.x, min.y, min.z, 1.),
        Vector4f::new(max.x, min.y, min.z, 1.),
        Vector4f::new(min.x, max.y, min.z, 1.),
        Vector4f::new(max.x, max.y, min.z, 1.),
        Vector4f::new(min.x, min.y, max.z, 1.),
        Vector4f::new(max.x, min.y, max.z, 1.),
        Vector4f::new(min.x, max.y, max.z, 1.),
        Vector4f::new(max.x, max.y, max.z, 1.),
    ] {
        let v = matrix * p;
        rv.update(&Vector3f::new(v.x, v.y, v.z));
    }
    rv
}

fn get_occlusion_projection_matrix(cube: &CuboidLike, view_matrix_camera: &Matrix4<f32>) -> Matrix4<f32> {
    // transform cube to view space and compute bounding box in view space
    let bounding_box = get_bounding_box(&cube.min(), &cube.max(), &view_matrix_camera);

    // compute perspective matrix
    let min = bounding_box.min();
    let max = bounding_box.max();
    Matrix4::from(Perspective{left: min.x, right: max.x, bottom: min.y, top: max.y, near: -min.z, far: 10000.})
}

fn draw_octree_view(_outlined_box_drawer: &OutlinedBoxDrawer, _camera: &Camera, _camera_octree: &Camera, _visible_nodes: &Vec<octree::VisibleNode>, _node_views: &mut NodeViewContainer, enable_occ_query: bool)
{
    unsafe {
        let x = _camera_octree.width;
        let y = 0;
        gl::Viewport(x, y, _camera_octree.width, _camera_octree.height);
        gl::Scissor(x, y, _camera_octree.width, _camera_octree.height);
        gl::Enable(gl::SCISSOR_TEST);

        gl::ClearColor(0.3, 0.3, 0.4, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }

    let color_table = [
        vec![1.,0.,0.,1.],
        vec![0.,1.,0.,1.],
        vec![0.,0.,1.,1.],
        vec![1.,1.,0.,1.],
        vec![0.,1.,1.,1.],
        vec![1.,0.,1.,1.],
        ];

    let mx_camera_octree: Matrix4<f32> = _camera_octree.get_world_to_gl();
    for visible_node in _visible_nodes {
        if let Some(view) = _node_views.get(&visible_node.id) {
            let mut color = &color_table[3];//visible_node.slice as usize % color_table.len()];
            if enable_occ_query {
                //if visible_node.occluder {
                    //color = &color_table[0];
                    //draw_outlined_box(&_outlined_box_drawer, &mx_camera_octree, view, &color);

                    // if draw_occ_frustums {
                    //     let view_matrix_camera = &_camera.get_world_to_camera();

                    //     let occ_projection_matrix = get_occlusion_projection_matrix(&view.meta.bounding_cube, view_matrix_camera);

                    //     let mx_occ_frustum = occ_projection_matrix * view_matrix_camera;
                    //     let mx_occ_frustum_inv: Matrix4<f32> = mx_occ_frustum.inverse_transform().unwrap().into();
                    //     let mx = mx_camera_octree * mx_occ_frustum_inv;

                    //     // frustum
                    //     _outlined_box_drawer.update_color(&color_table[5]);
                    //     _outlined_box_drawer.update_transform(&mx);
                    //     _outlined_box_drawer.draw();
                    // }
                //}
                //else 
                if visible_node.occluded {
                    //color = &color_table[4];
                    //draw_outlined_box(&_outlined_box_drawer, &mx_camera_octree, view, &color);
                } else {
                    color = &color_table[3];
                    draw_outlined_box(&_outlined_box_drawer, &mx_camera_octree, view, &color);           
                }
            } else {
                color = &color_table[3];
                draw_outlined_box(&_outlined_box_drawer, &mx_camera_octree, view, &color); 
            }
        }
    }
    
    // frustum
    let color = vec![1.,0.,1.,1.,1.];
    _outlined_box_drawer.update_color(&color);
    let mx_inv_camera:  Matrix4<f32> = _camera.get_world_to_gl().inverse_transform().unwrap().into();
    let mx = mx_camera_octree * mx_inv_camera;
    _outlined_box_drawer.update_transform(&mx);
    _outlined_box_drawer.draw();

    unsafe {
        gl::Disable(gl::SCISSOR_TEST);
        gl::Scissor(0, 0, _camera.width, _camera.height);
        gl::Viewport(0, 0, _camera.width, _camera.height);
    }
}

fn draw_outlined_box(outlined_box_drawer: &OutlinedBoxDrawer, projection_view_matrix: &Matrix4<f32>, node_view: &NodeView, color: &Vec<f32>)
{
    let half_edge_length = node_view.meta.bounding_cube.edge_length() / 2.0;
    let min_cube_pos = node_view.meta.bounding_cube.min();

    // create scale matrix   
    let mx_scale = Matrix4::from_scale(half_edge_length);
    
    // create translation matrix
    let half_edge_vector = Vector3::new(half_edge_length,half_edge_length,half_edge_length);
    let mx_translation = Matrix4::from_translation(min_cube_pos + half_edge_vector);
    
    let mx = projection_view_matrix * mx_translation * mx_scale;
    outlined_box_drawer.update_transform(&mx);

    outlined_box_drawer.update_color(&color);

    outlined_box_drawer.draw();
}

fn reshuffle(new_order: &[usize], old_data: Vec<u8>, bytes_per_vertex: usize) -> Vec<u8> {
    assert_eq!(new_order.len() * bytes_per_vertex, old_data.len());
    let mut new_data = Vec::with_capacity(old_data.len());
    for point_index in new_order {
        let i = point_index * bytes_per_vertex;
        new_data.extend(&old_data[i..i + bytes_per_vertex]);
    }
    assert_eq!(old_data.len(), new_data.len());
    new_data
}

struct NodeDrawer {
    program: GlProgram,

    // Uniforms locations.
    u_world_to_gl: GLint,
    u_edge_length: GLint,
    u_size: GLint,
    u_gamma: GLint,
    u_min: GLint,
}

impl NodeDrawer {
    fn new() -> Self {
        let program = GlProgram::new(VERTEX_SHADER_POINTS, FRAGMENT_SHADER_POINTS);
        let u_world_to_gl;
        let u_edge_length;
        let u_size;
        let u_gamma;
        let u_min;
        unsafe {
            gl::UseProgram(program.id);

            u_world_to_gl = gl::GetUniformLocation(program.id, c_str!("world_to_gl"));
            u_edge_length = gl::GetUniformLocation(program.id, c_str!("edge_length"));
            u_size = gl::GetUniformLocation(program.id, c_str!("size"));
            u_gamma = gl::GetUniformLocation(program.id, c_str!("gamma"));
            u_min = gl::GetUniformLocation(program.id, c_str!("min"));
        }
        NodeDrawer {
            program,
            u_world_to_gl,
            u_edge_length,
            u_size,
            u_gamma,
            u_min,
        }
    }

    fn update_world_to_gl(&self, matrix: &Matrix4<f32>) {
        unsafe {
            gl::UseProgram(self.program.id);            
            gl::UniformMatrix4fv(self.u_world_to_gl, 1, false as GLboolean, matrix.as_ptr());
        }
    }

    fn draw(&self, node_view: &NodeView, level_of_detail: i32, point_size: f32, gamma: f32) -> i64 {
        node_view.vertex_array.bind();
        let num_points = node_view
            .meta
            .num_points_for_level_of_detail(level_of_detail);
        unsafe {
            gl::UseProgram(self.program.id);
            gl::Enable(gl::PROGRAM_POINT_SIZE);
            gl::Enable(gl::DEPTH_TEST);

            gl::Uniform1f(
                self.u_edge_length,
                node_view.meta.bounding_cube.edge_length(),
            );
            gl::Uniform1f( self.u_size, point_size);
            gl::Uniform1f( self.u_gamma, gamma);
            gl::Uniform3fv(self.u_min, 1, node_view.meta.bounding_cube.min().as_ptr());

            gl::DrawArrays(gl::POINTS, 0, num_points as i32);

            gl::Disable(gl::PROGRAM_POINT_SIZE);
        }
        num_points
    }
}

struct NodeView {
    meta: octree::NodeMeta,

    // The buffers are bound by 'vertex_array', so we never refer to them. But they must outlive
    // this 'NodeView'.
    vertex_array: GlVertexArray,
    _buffer_position: GlBuffer,
    _buffer_color: GlBuffer,
}

impl NodeView {
    fn new(program: &GlProgram, node_data: octree::NodeData) -> Self {
        unsafe{
            gl::UseProgram(program.id);
        }

        let vertex_array = GlVertexArray::new();
        vertex_array.bind();

        // We draw the points in random order. This allows us to only draw the first N if we want
        // to draw less.
        let mut indices: Vec<usize> = (0..node_data.meta.num_points as usize).collect();
        let mut rng = thread_rng();
        rng.shuffle(&mut indices);

        let position = reshuffle(
            &indices,
            node_data.position,
            match node_data.meta.position_encoding {
                octree::PositionEncoding::Uint8 => 3,
                octree::PositionEncoding::Uint16 => 6,
                octree::PositionEncoding::Float32 => 12,
            },
        );
        let color = reshuffle(&indices, node_data.color, 3);

        let buffer_position = GlBuffer::new();
        let buffer_color = GlBuffer::new();

        unsafe {
            buffer_position.bind(gl::ARRAY_BUFFER);
            let (normalize, data_type) = match node_data.meta.position_encoding {
                octree::PositionEncoding::Uint8 => (true, gl::UNSIGNED_BYTE),
                octree::PositionEncoding::Uint16 => (true, gl::UNSIGNED_SHORT),
                octree::PositionEncoding::Float32 => (false, gl::FLOAT),
            };
            gl::BufferData(
                gl::ARRAY_BUFFER,
                position.len() as GLsizeiptr,
                mem::transmute(&position[0]),
                gl::STATIC_DRAW,
            );

            // Specify the layout of the vertex data.
            let pos_attr = gl::GetAttribLocation(program.id, c_str!("position"));
            gl::EnableVertexAttribArray(pos_attr as GLuint);
            gl::VertexAttribPointer(
                pos_attr as GLuint,
                3,
                data_type,
                normalize as GLboolean,
                0,
                ptr::null(),
            );

            buffer_color.bind(gl::ARRAY_BUFFER);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                color.len() as GLsizeiptr,
                mem::transmute(&color[0]),
                gl::STATIC_DRAW,
            );
            let color_attr = gl::GetAttribLocation(program.id, c_str!("color"));
            gl::EnableVertexAttribArray(color_attr as GLuint);
            gl::VertexAttribPointer(
                color_attr as GLuint,
                3,
                gl::UNSIGNED_BYTE,
                gl::FALSE as GLboolean,
                0,
                ptr::null(),
            );
        }
        NodeView {
            vertex_array,
            _buffer_position: buffer_position,
            _buffer_color: buffer_color,
            meta: node_data.meta,
        }
    }
}

// Keeps track of the nodes that were requested in-order and loads then one by one on request.
struct NodeViewContainer {
    node_views: HashMap<octree::NodeId, NodeView>,
    queue: VecDeque<octree::NodeId>,
    queued: HashSet<octree::NodeId>,
}

impl NodeViewContainer {
    fn new() -> Self {
        NodeViewContainer {
            node_views: HashMap::new(),
            queue: VecDeque::new(),
            queued: HashSet::new(),
        }
    }

    // Loads the next most-important nodes data. Returns false if there are no more nodes queued
    // for loading.
    fn load_next_node(&mut self, octree: &octree::Octree, program: &GlProgram) -> bool {
        // We always request nodes at full resolution (i.e. not subsampled by the backend), because
        // we can just as effectively subsample the number of points we draw in the client.
        const ALL_POINTS_LOD: i32 = 1;
        if let Some(node_id) = self.queue.pop_front() {
            self.queued.remove(&node_id);
            let node_data = octree.get_node_data(&node_id, ALL_POINTS_LOD).unwrap();
            self.node_views
                .insert(node_id, NodeView::new(program, node_data));
            true
        } else {
            false
        }

        // TODO(sirver): Use a LRU Cache to throw nodes out that we haven't used in a while.
    }

    fn reset_load_queue(&mut self) {
        self.queue.clear();
        self.queued.clear();
    }

    // Returns the 'NodeView' for 'node_id' if it is already loaded, otherwise returns None, but
    // registered the node for loading.
    fn get(&mut self, node_id: &octree::NodeId) -> Option<&NodeView> {
        match self.node_views.entry(*node_id) {
            Entry::Vacant(_) => {
                if !self.queued.contains(&node_id) {
                    self.queue.push_back(*node_id);
                    self.queued.insert(*node_id);
                }
                None
            }
            Entry::Occupied(e) => Some(e.into_mut()),
        }
    }
}

fn main() {
    let matches = clap::App::new("sdl_viewer")
        .args(
            &[
                clap::Arg::with_name("octree_directory")
                    .help("Input directory of the octree directory to serve.")
                    .index(1)
                    .required(true),
            ]
        )
        .get_matches();

    let octree_directory = PathBuf::from(matches.value_of("octree_directory").unwrap());
    let octree = octree::Octree::new(&octree_directory).unwrap();

    let ctx = sdl2::init().unwrap();
    let video_subsystem = ctx.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();

    // TODO(hrapp): This should use OpenGL ES 2.0 to be compatible with WebGL, so this can be made
    // to work with emscripten.
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 2);

    const WINDOW_WIDTH: i32 = 800;
    const WINDOW_HEIGHT: i32 = 600;
    let window = match video_subsystem
              .window("sdl2_viewer", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
              .position_centered()
              .resizable()
              .opengl()
              .build() {
        Ok(window) => window,
        Err(err) => panic!("failed to create window: {}", err),
    };

    // We need to create a context now, only after can we actually legally load the gl functions
    // and query 'gl_attr'.
    let _context = window.gl_create_context().unwrap();
    video_subsystem.gl_set_swap_interval(1);

    assert_eq!(gl_attr.context_profile(), GLProfile::Core);

    gl::load_with(
        |s| {
            let ptr = video_subsystem.gl_get_proc_address(s);
            unsafe { std::mem::transmute(ptr) }
        }
    );

    let node_drawer = NodeDrawer::new();
    let mut node_views = NodeViewContainer::new();
    let mut visible_nodes = Vec::new();

    let outlined_box_drawer = OutlinedBoxDrawer::new();
    let quad_drawer = QuadDrawer::new();

    let mut camera = Camera::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    camera.set_pos_rot(&Vector3::new(-4., 8.5, 1.), Deg(90.), Deg(90.));
    let mut camera_octree = Camera::new(WINDOW_WIDTH / 2, WINDOW_HEIGHT / 2);

    let mut gl_query = GlQuery::new();
    let mut gl_query_node = GlQuery::new();
    let mut enable_occ_query = false;
    let mut batch_size = 10;

    let mut gl_framebuffer = GlFramebuffer::new();

    let mut events = ctx.event_pump().unwrap();
    let mut num_frames = 0;
    let mut last_log = time::PreciseTime::now();
    let mut force_load_all = false;
    let mut show_octree_nodes = false;
    let mut show_octree_view = false;
    let mut use_level_of_detail = true;
    let mut point_size = 1.;
    let mut gamma = 1.3;
    let mut max_number_of_points_per_node = 0;
    let mut main_loop = || {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => process::exit(1),
                Event::KeyDown { scancode: Some(code), .. } => {
                    match code {
                        Scancode::Escape => process::exit(1),
                        Scancode::W => camera.moving_forward = true,
                        Scancode::S => camera.moving_backward = true,
                        Scancode::A => camera.moving_left = true,
                        Scancode::D => camera.moving_right = true,
                        Scancode::Z => camera.moving_down = true,
                        Scancode::Q => camera.moving_up = true,
                        Scancode::F => force_load_all = true,
                        Scancode::O => show_octree_nodes = !show_octree_nodes,
                        Scancode::P => show_octree_view = !show_octree_view,
                        Scancode::Num7 => gamma -= 0.1,
                        Scancode::Num8 => gamma += 0.1,
                        Scancode::Num9 => point_size -= 0.1,
                        Scancode::Num0 => point_size += 0.1,
                        Scancode::B => enable_occ_query = !enable_occ_query,
                        Scancode::N => { batch_size -= 1; println!("batch_size {}", batch_size) },
                        Scancode::M => { batch_size += 1; println!("batch_size {}", batch_size) },
                        _ => (), 
                    }
                }
                Event::KeyUp { scancode: Some(code), .. } => {
                    match code {
                        Scancode::W => camera.moving_forward = false,
                        Scancode::S => camera.moving_backward = false,
                        Scancode::A => camera.moving_left = false,
                        Scancode::D => camera.moving_right = false,
                        Scancode::Z => camera.moving_down = false,
                        Scancode::Q => camera.moving_up = false,
                        _ => (),
                    }
                }
                Event::MouseMotion {
                    xrel,
                    yrel,
                    mousestate,
                    ..
                } if mousestate.left() => camera.mouse_drag(xrel, yrel),
                Event::MouseWheel { y, .. } => {
                    camera.mouse_wheel(y);
                }
                Event::Window { win_event: WindowEvent::SizeChanged(w, h), .. } => {
                    camera.set_size(w, h);
                    camera_octree.set_size(w / 2, h / 2);
                }
                _ => (),
            }
        }

        if camera.update() {
            use_level_of_detail = false;
            node_drawer.update_world_to_gl(&camera.get_world_to_gl());
            visible_nodes = octree.get_visible_nodes(
                &camera.get_world_to_gl(),
                camera.width,
                camera.height,
                octree::UseLod::Yes,
            );
            node_views.reset_load_queue();
        } else {
            use_level_of_detail = false;
        }

        let mut num_points_drawn = 0;
        let mut num_nodes_drawn = 0;

        let slice_pixel_count: i64 = camera.width as i64 * camera.height as i64;
        let mut current_slice_pixel_count = 0;

        gl_query.begin_samples_passed();

        let mut current_batch = 0;
        let mut num_queries = 0;

        //gl_framebuffer.bind();

        unsafe {
            gl::Viewport(0, 0, camera.width, camera.height);
            gl::ClearColor(0., 0., 0., 1.);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            if !enable_occ_query {
                for i in 0..visible_nodes.len() {
                    let visible_node = &mut visible_nodes[i];

                    if let Some(view) = node_views.get(&visible_node.id) {

                        let node_points_drawn = node_drawer.draw(
                            view,
                            if use_level_of_detail {
                                visible_node.level_of_detail
                            } else {
                                1
                            },
                            point_size, gamma
                        );

                        num_points_drawn += node_points_drawn;
                        num_nodes_drawn += 1;
                        if max_number_of_points_per_node < node_points_drawn {
                            max_number_of_points_per_node = node_points_drawn;
                        }
                        if show_octree_nodes {
                            let color_intensity = num_points_drawn as f32 / max_number_of_points_per_node as f32;
                            let color = vec![color_intensity,color_intensity,0.,1.];
                            draw_outlined_box(&outlined_box_drawer, &camera.get_world_to_gl(), view, &color);
                        }
                        current_slice_pixel_count += node_points_drawn;
                    }
                }
            } else {
                // occ query pass
                let mut query_state = false;       // render state or query state
                let node_count = visible_nodes.len();
                for i in 0..node_count {
                    if visible_nodes[i].occluded {
                       continue;
                    }

                    if let Some(view) = node_views.get(&visible_nodes[i].id) {

                        if query_state {
                            gl_query_node.begin_samples_passed();   
                        }

                        let node_points_drawn = node_drawer.draw(
                            view,
                            if use_level_of_detail {
                                visible_nodes[i].level_of_detail
                            } else {
                                1
                            },
                            point_size, gamma
                        );

                        if query_state {
                            gl_query_node.end();
                            let samples_passed = gl_query_node.query_samples_passed();

                            let pass_ratio = samples_passed as f32 / node_points_drawn as f32;
                            if pass_ratio < 0.001 {

                                visible_nodes[i].occluder = true;

                                let world_to_camera_matrix = camera.get_world_to_camera();
                                let occ_projection_matrix = get_occlusion_projection_matrix(&view.meta.bounding_cube, &world_to_camera_matrix);
                                let mx = occ_projection_matrix * world_to_camera_matrix;
                                let frustum = Frustum::from_matrix(&mx);

                                for j in (i+1)..node_count {
                                    if visible_nodes[j].occluder {          // this should never happen
                                        continue;
                                    }
                                    if visible_nodes[j].occluded {
                                       continue;
                                    }
                                    if frustum.intersects_inside_or_intersect(&visible_nodes[j].bounding_cube) {
                                        visible_nodes[j].occluded = true;
                                    }
                                }
                            }
                            num_queries += 1;
                        }

                        num_points_drawn += node_points_drawn;
                        num_nodes_drawn += 1;
                        if show_octree_nodes {
                            let color_intensity = 1.;
                            let color = vec![color_intensity,color_intensity,0.,1.];
                            draw_outlined_box(&outlined_box_drawer, &camera.get_world_to_gl(), view, &color);
                        }
                        current_slice_pixel_count += node_points_drawn;

                        if current_slice_pixel_count >= slice_pixel_count {
                            current_batch += 1;
                            current_slice_pixel_count = 0;

                            if !query_state {
                                if current_batch >= batch_size {
                                    query_state = true;
                                }
                            } else {
                                // finish query state after one batch
                                query_state = false;
                                
                                //break;
                            }
                        }
                    }
                }
            }
        }
        //gl_framebuffer.unbind();


        quad_drawer.draw();

        gl_query.end();

        if force_load_all {
            println!("Force loading all currently visible nodes.");
            while node_views.load_next_node(&octree, &node_drawer.program) {}
            force_load_all = false;
        } else {
            // TODO(happ): this is arbitrary - how fast should we load stuff?
            for _ in 0..10 {
                node_views.load_next_node(&octree, &node_drawer.program);
            }
        }

        if show_octree_view {
            draw_octree_view(&outlined_box_drawer, &camera, &camera_octree, &visible_nodes, &mut node_views, enable_occ_query);
        }

        window.gl_swap_window();

        let samples_passed = gl_query.query_samples_passed();

        let err;
        unsafe {
            err = gl::GetError();
        }

        num_frames += 1;
        let now = time::PreciseTime::now();
        if last_log.to(now) > time::Duration::seconds(1) {
            let duration = last_log.to(now).num_microseconds().unwrap();
            let fps = (num_frames * 1_000_000u32) as f32 / duration as f32;
            num_frames = 0;
            last_log = now;
            println!(
                "FPS: {:#?}, Drew {} / {} ({}%) points. total nodes {}, nodes drawm {}, queries {}, glerror {}",
                fps,
                samples_passed, num_points_drawn, samples_passed as f32 / num_points_drawn as f32 * 100.,
                visible_nodes.len(),
                num_nodes_drawn,
                num_queries,
                err,
            );
        }
    };

    loop {
        main_loop();
    }
}
