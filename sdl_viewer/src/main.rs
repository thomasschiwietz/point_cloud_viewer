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
use point_viewer::math::{Cuboid, CuboidLike, Cube, Frustum, Vector2f, Vector3f, Vector4f, Matrix4f};
use point_viewer::octree;
use rand::{Rng, thread_rng};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Scancode;
use sdl2::video::GLProfile;
use sdl_viewer::{Camera, gl};
use sdl_viewer::boxdrawer::BoxDrawer;
use sdl_viewer::zbuffer_drawer::ZBufferDrawer;
use sdl_viewer::reduction::Reduction;
use sdl_viewer::gl::types::{GLboolean, GLint, GLsizeiptr, GLuint};
use sdl_viewer::graphic::{GlBuffer, GlProgram, GlVertexArray, GlQuery, GlTexture, TextureType};
use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::cmp;
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

    // The OpenGL coordinate system z-axis has a negative sign from the camera point in the viewing direction.
    // min.z/max.z must be flipped as parameters for near/far in the projection matrix
    // The near and far plane enclosing the box are in [-max.z;-min.z]
    // Matrix4::from(Perspective{left: min.x, right: max.x, bottom: min.y, top: max.y, near: -max.z, far: 10000.})    

    // We set the near plane of the frustum to the back plane of the box at -min.z
    // The left/right/top/bottom planes must be adapted to account for the perspective

    let z_ratio = min.z / max.z;
    let min_xy_at_min_z = Vector2f::new(
        min.x * z_ratio,
        min.y * z_ratio,
    );
    let max_xy_at_min_z = Vector2f::new(
        max.x * z_ratio,
        max.y * z_ratio,
    );

    let mut min_xy_frustum = min_xy_at_min_z;
    let mut max_xy_frustum = max_xy_at_min_z;
    if min.x < min_xy_frustum.x { min_xy_frustum.x = min.x; }
    if min.y < min_xy_frustum.y { min_xy_frustum.y = min.y; }
    if max.x > max_xy_frustum.x { max_xy_frustum.x = max.x; }
    if max.y > max_xy_frustum.y { max_xy_frustum.y = max.y; }

    Matrix4::from(Perspective{left: min_xy_frustum.x, right: max_xy_frustum.x, bottom: min_xy_frustum.y, top: max_xy_frustum.y, near: -min.z, far: 10000.})
}

fn draw_octree_view(box_drawer: &BoxDrawer, camera: &Camera, camera_octree: &Camera, visible_nodes: &Vec<octree::VisibleNode>, occlusion_world_to_proj_matrices: &Vec<Matrix4f>, node_views: &mut NodeViewContainer, node_drawer: &NodeDrawer)
{
    unsafe {
        let x = camera.width - camera_octree.width;
        let y = 0;
        gl::Viewport(x, y, camera_octree.width, camera_octree.height);
        gl::Scissor(x, y, camera_octree.width, camera_octree.height);
        gl::Enable(gl::SCISSOR_TEST);

        gl::ClearColor(0.3, 0.3, 0.4, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

        gl::Disable(gl::DEPTH_TEST);

        //gl::Enable(gl::LINE_SMOOTH);
        //gl::LineWidth(0.5);
    }

    // let color_table = [
    //     vec![1.,0.,0.,1.],
    //     vec![0.,1.,0.,1.],
    //     vec![0.,0.,1.,1.],
    //     vec![1.,1.,0.,1.],
    //     vec![0.,1.,1.,1.],
    //     vec![1.,0.,1.,1.],
    // ];

    // plane is fixed
    let mut mx_camera_octree: Matrix4<f32> = camera_octree.get_world_to_gl();

    // frustum is fixed
    // mx_camera_octree = camera_octree.get_projection_matrix() * Matrix4f::from_angle_x(Rad::from(Deg(90.))) * camera.get_world_to_camera();

    // -1. render grid lines 25 m spacing
    {
        let color = vec![0.2, 0.2, 0.3, 1.0];
        box_drawer.update_color(&color);
        for i in 1..5 {
            let mx = camera_octree.get_projection_matrix() * Matrix4f::from_nonuniform_scale(200.0, i as f32 * 25.0, 1.);
            box_drawer.update_transform(&mx);
            box_drawer.draw_outlines();
        }
    }


    // 0. render all nodes
    node_drawer.update_world_to_gl(&mx_camera_octree);
    for visible_node in visible_nodes {
        if visible_node.drawn {
            if let Some(view) = node_views.get(&visible_node.id) {
                node_drawer.draw(view, 1, 1.0, 1.3);
            }
        }
    }
    node_drawer.update_world_to_gl(&camera.get_world_to_gl());
    unsafe {
        gl::Disable(gl::DEPTH_TEST);        
    }

    // 1. render all nodes in dark gray
    for visible_node in visible_nodes {
        if !visible_node.occluder {
            draw_outlined_box(&box_drawer, &mx_camera_octree, &visible_node.bounding_cube, &vec![0.2,0.2,0.2,1.]);
        }
    }

    // 3. occlusion frustums
    for occ_world_to_proj_matrix in occlusion_world_to_proj_matrices {
        let color = vec![1.,0.,1.,1.,1.];
        box_drawer.update_color(&color);
        let mx_inv_frustum:  Matrix4<f32> = occ_world_to_proj_matrix.inverse_transform().unwrap().into();
        let mx = mx_camera_octree * mx_inv_frustum;
        box_drawer.update_transform(&mx);
        box_drawer.draw_outlines();
    }

    // 4. render all occluded nodes
    for visible_node in visible_nodes {
        if visible_node.occluded {
            draw_outlined_box(&box_drawer, &mx_camera_octree, &visible_node.bounding_cube, &vec![0.75,0.2,0.2,1.]); 
        }
    }

    // 5. render all occluder nodes
    unsafe {
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
    }
    for visible_node in visible_nodes {
        if visible_node.occluder {
            draw_filled_box(&box_drawer, &camera_octree.get_world_to_camera(), &mx_camera_octree, &visible_node.bounding_cube, &vec![0.5,0.2,0.2,1.]); 
        }
    }
    unsafe {
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::DEPTH_TEST);
    }
    for visible_node in visible_nodes {
        if visible_node.occluder {
            draw_outlined_box(&box_drawer, &mx_camera_octree, &visible_node.bounding_cube, &vec![0.75,0.2,0.2,1.]); 
        }
    }

    // 2. render all drawn nodes in green
    for visible_node in visible_nodes {
        if visible_node.drawn {
            draw_outlined_box(&box_drawer, &mx_camera_octree, &visible_node.bounding_cube, &vec![0.2,0.75,0.2,1.]);
        }
    }

    // viewing frustum
    let color = vec![1.,1.,1.,1.];
    box_drawer.update_color(&color);
    let mx_camera:  Matrix4<f32> = camera.get_world_to_gl();
    let mx_inv_camera:  Matrix4<f32> = mx_camera.inverse_transform().unwrap().into();
    let mx = mx_camera_octree * mx_inv_camera;
    box_drawer.update_transform(&mx);
    box_drawer.draw_outlines();

    // render border
    let border_color = vec![0.5, 0.5, 0.6, 1.0];
    let mx = Matrix4f::from_nonuniform_scale(1. - (1. / camera_octree.width as f32), 1. - (1. / camera_octree.height as f32), 1.);
    box_drawer.update_transform(&mx);
    box_drawer.update_color(&border_color);
    box_drawer.draw_outlines();

    unsafe {
        //gl::LineWidth(1.0);
        //gl::Disable(gl::LINE_SMOOTH);    

        gl::Enable(gl::DEPTH_TEST);        
        gl::Disable(gl::SCISSOR_TEST);
        gl::Scissor(0, 0, camera.width, camera.height);
        gl::Viewport(0, 0, camera.width, camera.height);
    }
}

fn draw_outlined_box(box_drawer: &BoxDrawer, projection_view_matrix: &Matrix4<f32>, cube: &Cube, color: &Vec<f32>)
{
    // create scale matrix   
    let mx_scale = Matrix4::from_scale(cube.edge_length() / 2.);
    
    // create translation matrix
    let mx_translation = Matrix4::from_translation(cube.center());
    
    let mx = projection_view_matrix * mx_translation * mx_scale;
    box_drawer.update_transform(&mx);

    box_drawer.update_color(&color);

    box_drawer.draw_outlines();
}

fn draw_filled_box(box_drawer: &BoxDrawer, mx_world_to_camera: &Matrix4f, projection_view_matrix: &Matrix4<f32>, cube: &Cube, color: &Vec<f32>)
{
    // create scale matrix   
    let mx_scale = Matrix4::from_scale(cube.edge_length() / 2.);
    
    // create translation matrix
    let mx_translation = Matrix4::from_translation(cube.center());
    
    let mx = projection_view_matrix * mx_translation * mx_scale;

    box_drawer.draw_filled(&color, &mx_world_to_camera, &mx);
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

enum RenderMode {
    BruteForce,
    Limited,
    OcclusionQuery,
    OcclusionQuerySkipSmallFrustums,
    OcclusionAsncQuery,
    ZBuffer,
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

    let box_drawer = BoxDrawer::new();
    let zbuffer_drawer = ZBufferDrawer::new();
    let mut reduction = Reduction::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    let mut max_reduce_steps = 1;
    let mut reduction_limit = 4;

    let mut camera = Camera::new(WINDOW_WIDTH, WINDOW_HEIGHT, false);
    camera.set_pos_rot(&Vector3::new(-4., 8.5, 1.), Deg(90.), Deg(90.));

    let mut octree_view_size = 0.5;
    let mut camera_octree = Camera::new((WINDOW_WIDTH as f32 * octree_view_size) as i32, (WINDOW_HEIGHT as f32 * octree_view_size) as i32, true);

    let mut gl_query = GlQuery::new();
    let mut gl_query_node = GlQuery::new();
    let mut batch_size = 30;

    let mut render_mode = RenderMode::BruteForce;

    let mut slice_limit = 20;

    let mut show_depth_buffer = false;
    let mut show_reduced_depth_buffer = false;
    let mut gl_depth_texture = GlTexture::new(camera.width, camera.height, TextureType::Depth);

    let mut shift_pressed = false;

    let mut events = ctx.event_pump().unwrap();
    let mut num_frames = 0;
    let mut last_log = time::PreciseTime::now();
    let mut force_load_all = false;
    let mut show_octree_nodes = false;
    let mut show_octree_view = false;
    let mut use_level_of_detail = true;
    let mut point_size = 1.;
    let mut gamma = 1.3;
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
                        Scancode::P => { 
                            show_octree_view = !show_octree_view; 
                            if show_octree_view {
                                if shift_pressed {
                                    octree_view_size = 1.0;
                                } else { 
                                    octree_view_size = 0.5; 
                                }
                                camera_octree.set_size((camera.width as f32 * octree_view_size) as i32, (camera.height as f32 * octree_view_size) as i32);
                            }
                        },
                        Scancode::Num1 => { render_mode = RenderMode::BruteForce; println!("render mode: brute force"); },
                        Scancode::Num2 => { render_mode = RenderMode::Limited; println!("render mode: limited"); },
                        Scancode::Num3 => { render_mode = RenderMode::OcclusionQuery; println!("render mode: occlusion query"); },
                        Scancode::Num4 => { render_mode = RenderMode::OcclusionQuerySkipSmallFrustums; println!("render mode: occlusion query: skip small frustums"); },
                        Scancode::Num5 => { render_mode = RenderMode::OcclusionAsncQuery; println!("render mode: occlusion query: async"); },
                        Scancode::Num6 => { render_mode = RenderMode::ZBuffer; println!("render mode: zbuffer"); },
                        Scancode::Num7 => gamma -= 0.1,
                        Scancode::Num8 => gamma += 0.1,
                        Scancode::Num9 => point_size -= 0.1,
                        Scancode::Num0 => point_size += 0.1,
                        Scancode::X => show_depth_buffer = !show_depth_buffer,
                        Scancode::C => show_reduced_depth_buffer = !show_reduced_depth_buffer,
                        Scancode::V => { max_reduce_steps -= 1; max_reduce_steps = cmp::max(max_reduce_steps, 1); println!("max_reduce_steps {}", max_reduce_steps) },
                        Scancode::B => { max_reduce_steps += 1; println!("max_reduce_steps {}", max_reduce_steps) },
                        Scancode::N => { batch_size -= 1; println!("batch_size {}", batch_size) },
                        Scancode::M => { batch_size += 1; println!("batch_size {}", batch_size) },
                        Scancode::H => { slice_limit -= 1; slice_limit = cmp::max(slice_limit, 1); println!("slice_limit {}", slice_limit) },
                        Scancode::J => { slice_limit += 1; println!("slice_limit {}", slice_limit) },
                        Scancode::LShift => shift_pressed = true,
                        Scancode::U => { reduction_limit /= 2; println!("reduction_limit {}", reduction_limit) },
                        Scancode::I => { reduction_limit *= 2; println!("reduction_limit {}", reduction_limit) },
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
                        Scancode::LShift => shift_pressed = false,
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
                    camera_octree.set_size((w as f32 * octree_view_size) as i32, (h as f32 * octree_view_size) as i32);
                    gl_depth_texture.set_size(w, h);
                    reduction.set_size(w, h);
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
            camera_octree.set_pos_rot(&camera.get_pos(), Deg::from(camera.get_theta()), Deg(0.));
            camera_octree.update();
        } else {
            use_level_of_detail = false;

            // clear occlusion
            for i in 0..visible_nodes.len() {
                visible_nodes[i].drawn = false;
                visible_nodes[i].occluder = false;
                visible_nodes[i].occluded = false;
            } 
        }

        let mut num_points_drawn = 0;
        let mut num_nodes_drawn = 0;

        let num_screen_space_pixels: i64 = camera.width as i64 * camera.height as i64;
        let mut current_num_screen_space_pixels = 0;
        let mut slice_count = 0;

        let mut occlusion_world_to_proj_matrices = Vec::new();

        //gl_query.begin_samples_passed();

        let mut current_batch = 0;
        let mut num_queries = 0;

        unsafe {
            gl::Viewport(0, 0, camera.width, camera.height);
            gl::ClearColor(0., 0., 0., 1.);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        match render_mode {
            RenderMode::BruteForce => {
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

                        visible_node.drawn = true;

                        num_points_drawn += node_points_drawn;
                        num_nodes_drawn += 1;
                    }
                }
            },
            RenderMode::Limited => {
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

                        visible_node.drawn = true;

                        num_points_drawn += node_points_drawn;
                        num_nodes_drawn += 1;
                        current_num_screen_space_pixels += node_points_drawn;

                        if current_num_screen_space_pixels >= num_screen_space_pixels {
                            current_num_screen_space_pixels = 0;

                            slice_count += 1;
                            if slice_count >= slice_limit {
                                break;
                            }
                        }
                    }
                }
            },
            RenderMode::OcclusionQuery => {
                occlusion_world_to_proj_matrices.clear();                
                let mut query_state = false;       // render state or query state. should be an enum
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

                        visible_nodes[i].drawn = true;

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

                                // add occlusion frustum to debug view
                                //occlusion_world_to_proj_matrices.push(mx);                                

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
                        current_num_screen_space_pixels += node_points_drawn;

                        if current_num_screen_space_pixels >= num_screen_space_pixels {
                            current_batch += 1;
                            current_num_screen_space_pixels = 0;

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
            },
            RenderMode::ZBuffer => {
                // clear occlusion
                for i in 0..visible_nodes.len() {
                    visible_nodes[i].drawn = false;
                    visible_nodes[i].occluder = false;
                    visible_nodes[i].occluded = false;
                } 

                occlusion_world_to_proj_matrices.clear();
                let node_count = visible_nodes.len();
                let mut cell_depth: Vec<f32> = Vec::new();
                cell_depth.resize(32 * 32, 1.0);          // remove hardcoded limit!
                for i in 0..node_count {
                    if visible_nodes[i].occluded {
                        continue;
                    }

                    if let Some(view) = node_views.get(&visible_nodes[i].id) {

                        let node_points_drawn = node_drawer.draw(
                            view,
                            if use_level_of_detail {
                                visible_nodes[i].level_of_detail
                            } else {
                                1
                            },
                            point_size, gamma
                        );

                        visible_nodes[i].drawn = true;

                        num_points_drawn += node_points_drawn;
                        num_nodes_drawn += 1;
                        current_num_screen_space_pixels += node_points_drawn;
                    }

                    if current_num_screen_space_pixels >= num_screen_space_pixels {
                        current_batch += 1;
                        current_num_screen_space_pixels = 0;

                        if current_batch == batch_size {
                            //current_batch = 0;
                            // copy depth buffer to texture
                            unsafe {
                                gl::BindTexture(gl::TEXTURE_2D, gl_depth_texture.id);
                                gl::ReadBuffer(gl::BACK);
                                gl::CopyTexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT, 0, 0, camera.width, camera.height, 0);
                            }

                            // reduce z-buffer
                            let (_texture_id, _tex_scale, framebuffer_id, width, height) = reduction.reduce_max(gl_depth_texture.id, camera.width, camera.height, 20, reduction_limit);

                            // download data
                            let data = reduction.download_data(framebuffer_id, width, height);

                            // bug: framebuffer must reset correct viewport
                            unsafe {
                                gl::Viewport(0, 0, camera.width, camera.height);
                            }

                            let projection_matrix = camera.get_projection_matrix();
                            let inv_projection_matrix: Matrix4<f32> = projection_matrix.inverse_transform().unwrap().into();

                            // create frustum for each z-buffer tile
                            let mut p = 0;
                            for y in 0..height {
                                for x in 0..width {
                                    let depth = data[p];


                                    // don't frustum cull cells that have already culled with a nearer near plane
                                    let linear_pos = (y * width + x) as usize;
                                    //println!("{}, {} -> {} / {}", x, y, depth, cell_depth[linear_pos]);
                                    if depth as f32 > cell_depth[linear_pos] as f32 {
                                        println!("skip: {},{}: depth {}, cell_depth {}", x,y, depth, cell_depth[linear_pos]);
                                        // why is this never happening?
                                        // why doesn't this work?
                                        //if depth >= 1.0 {
                                        //    continue;
                                        //}
                                        //println!("depth {}", depth);
                                        continue;
                                    }
                                    cell_depth[linear_pos] = depth;

                                    let mut cuboid = Cuboid::new();

                                    let mut proj_pos = Vec::new();

                                    // normalized coordinates in projection space for all four vertices of the tile
                                    proj_pos.push(Vector4f::new(
                                        (x as f32 + 0.0) / (width) as f32 * 2. - 1.,
                                        (y as f32 + 0.0) / (height) as f32 * 2. -1.,
                                        depth,
                                        1.));

                                    // proj_pos.push(Vector4f::new(
                                    //     (x as f32 + 0.0) / (width) as f32 * 2. - 1.,
                                    //     (y as f32 + 1.0) / (height) as f32 * 2. -1.,
                                    //     depth,
                                    //     1.));

                                    // proj_pos.push(Vector4f::new(
                                    //     (x as f32 + 0.0) / (width) as f32 * 2. - 1.,
                                    //     (y as f32 + 1.0) / (height) as f32 * 2. -1.,
                                    //     depth,
                                    //     1.));

                                    proj_pos.push(Vector4f::new(
                                        (x as f32 + 1.) / (width) as f32 * 2. - 1.,
                                        (y as f32 + 1.) / (height) as f32 * 2. -1.,
                                        depth,
                                        1.));

                                    // transform back to camera space
                                    for p in proj_pos.iter() {
                                        let camera_pos = inv_projection_matrix * p;

                                        let homo_pos = Vector3f::new(
                                            camera_pos.x / camera_pos.w,
                                            camera_pos.y / camera_pos.w,
                                            camera_pos.z / camera_pos.w,
                                        );

                                        cuboid.update(&homo_pos);
                                    }

                                    // compute perspective matrix
                                    let min = cuboid.min();
                                    let max = cuboid.max();
                                    if -min.z < 10000.0 {         
                                        let occ_proj_matrix = Matrix4::from(Perspective{left: min.x, right: max.x, bottom: min.y, top: max.y, near: -min.z, far: 10000.});
                                        let world_to_camera_matrix = camera.get_world_to_camera();
                                        let mx = occ_proj_matrix * world_to_camera_matrix;
                                        occlusion_world_to_proj_matrices.push(mx);
                                        let frustum = Frustum::from_matrix(&mx);

                                        for j in (i+1)..node_count {
                                        //for j in 0..node_count {
                                            if visible_nodes[j].occluded {
                                                continue;
                                            }
                                            if frustum.intersects_inside_or_intersect(&visible_nodes[j].bounding_cube) {
                                                visible_nodes[j].occluded = true;
                                                visible_nodes[j].occluder = true;
                                            }
                                        }
                                    }
                                    p = p + 1;                                    
                                }
                            }
                            //break;
                        }
                    }
                }
            },
            RenderMode::OcclusionQuerySkipSmallFrustums => {
                let mut max_edge_length = 0.;
                let mut min_edge_length = 10000000.;
                for visible_node in &visible_nodes {
                    let e = visible_node.bounding_cube.edge_length();
                    if e < min_edge_length { min_edge_length = e; }
                    if e > max_edge_length { max_edge_length = e; }
                }
                //println!("min/max {} x {}", min_edge_length, max_edge_length);

                occlusion_world_to_proj_matrices.clear();                
                let mut query_state = false;       // render state or query state. should be an enum
                let node_count = visible_nodes.len();
                for i in 0..node_count {
                    if visible_nodes[i].occluded {
                        continue;
                    }

                    if let Some(view) = node_views.get(&visible_nodes[i].id) {

                        if query_state && visible_nodes[i].bounding_cube.edge_length() >= 3. * min_edge_length {
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

                        visible_nodes[i].drawn = true;

                        if query_state && visible_nodes[i].bounding_cube.edge_length() >= 3. * min_edge_length {
                            gl_query_node.end();
                            let samples_passed = gl_query_node.query_samples_passed();

                            let pass_ratio = samples_passed as f32 / node_points_drawn as f32;
                            if pass_ratio < 0.001 {

                                visible_nodes[i].occluder = true;

                                let world_to_camera_matrix = camera.get_world_to_camera();
                                let occ_projection_matrix = get_occlusion_projection_matrix(&view.meta.bounding_cube, &world_to_camera_matrix);
                                let mx = occ_projection_matrix * world_to_camera_matrix;
                                let frustum = Frustum::from_matrix(&mx);

                                // add occlusion frustum to debug view
                                //occlusion_world_to_proj_matrices.push(mx);                                

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
                        current_num_screen_space_pixels += node_points_drawn;

                        if current_num_screen_space_pixels >= num_screen_space_pixels {
                            current_batch += 1;
                            current_num_screen_space_pixels = 0;

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
            },
            RenderMode::OcclusionAsncQuery => {
                let mut max_edge_length = 0.;
                let mut min_edge_length = 10000000.;
                for visible_node in &visible_nodes {
                    let e = visible_node.bounding_cube.edge_length();
                    if e < min_edge_length { min_edge_length = e; }
                    if e > max_edge_length { max_edge_length = e; }
                }
                //println!("min/max {} x {}", min_edge_length, max_edge_length);

                occlusion_world_to_proj_matrices.clear();

                let mut request_query = false;
                let mut query_submitted = false;
                let mut query_points_submitted = 0;
                let mut query_wait_for_result = false;
                let mut query_node_index = 0;

                let node_count = visible_nodes.len();
                for i in 0..node_count {
                    if visible_nodes[i].occluded {
                        continue;
                    }

                    if let Some(view) = node_views.get(&visible_nodes[i].id) {

                        if request_query && visible_nodes[i].bounding_cube.edge_length() >= 3. * min_edge_length {
                            gl_query_node.begin_samples_passed();
                            request_query = false;
                            query_submitted = true;
                            query_node_index = i;
                            println!("query node {}", i);
                        }

                        let node_points_submitted = node_drawer.draw(
                            view,
                            if use_level_of_detail {
                                visible_nodes[i].level_of_detail
                            } else {
                                1
                            },
                            point_size, gamma
                        );

                        if query_submitted {
                            gl_query_node.end();
                            query_points_submitted = node_points_submitted;
                            query_wait_for_result = true;
                        }

                        visible_nodes[i].drawn = true;

                        if query_wait_for_result {
                            if gl_query_node.is_result_available() {

                                println!("current {}, requested {}", i, query_node_index);

                                let samples_passed = gl_query_node.query_samples_passed();

                                let pass_ratio = samples_passed as f32 / query_points_submitted as f32;
                                if pass_ratio < 0.001 {

                                    visible_nodes[i].occluder = true;

                                    let world_to_camera_matrix = camera.get_world_to_camera();
                                    let occ_projection_matrix = get_occlusion_projection_matrix(&visible_nodes[query_node_index].bounding_cube, &world_to_camera_matrix);
                                    let mx = occ_projection_matrix * world_to_camera_matrix;
                                    let frustum = Frustum::from_matrix(&mx);

                                    // add occlusion frustum to debug view
                                    occlusion_world_to_proj_matrices.push(mx);                                

                                    for j in (query_node_index+1)..node_count {
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

                                query_wait_for_result = false;
                                query_submitted = false;

                                break;
                            }
                            //else {
                            //     println!("not available {} {} {}", request_query, query_submitted, query_wait_for_result);
                            // }
                        }

                        num_points_drawn += node_points_submitted;
                        num_nodes_drawn += 1;
                        current_num_screen_space_pixels += node_points_submitted;

                        if current_num_screen_space_pixels >= num_screen_space_pixels {
                            current_batch += 1;
                            current_num_screen_space_pixels = 0;

                            if !request_query && !query_submitted {
                                if current_batch >= batch_size {
                                    request_query = true;
                                }
                            } //else {
                                // finish query state after one batch
                                //query_state = false;
                                
                                //break;
                            //}
                        }
                    }
                }
            },
        }

        //gl_query.end();

        if show_octree_nodes {
            for v in &visible_nodes {
                let color_intensity = 1.;
                let color = vec![color_intensity,color_intensity,0.,1.];
                draw_outlined_box(&box_drawer, &camera.get_world_to_gl(), &v.bounding_cube, &color);
            }
        }

        if show_depth_buffer {
            // copy depth buffer to texture
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, gl_depth_texture.id);
                gl::ReadBuffer(gl::BACK);
                gl::CopyTexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT, 0, 0, camera.width, camera.height, 0);
            }

            if !show_reduced_depth_buffer {
                zbuffer_drawer.draw(gl_depth_texture.id, 1., 1.);
            } else {
                let (texture_id, tex_scale, framebuffer_id, width, height) = reduction.reduce_max(gl_depth_texture.id, camera.width, camera.height, max_reduce_steps, 8);

                zbuffer_drawer.draw(texture_id, tex_scale, tex_scale);

                if width <= 8 || height <= 8 {
                    // download data
                    let data = reduction.download_data(framebuffer_id, width, height);

                    println!("depth data {} x {}", width, height);

                    let projection_matrix = camera.get_projection_matrix();
                    let inv_projection_matrix: Matrix4<f32> = projection_matrix.inverse_transform().unwrap().into();

                    let mut i = 0;
                    for y in 0..height {
                        for x in 0..width {
                            let depth = data[i];

                            // normalized projection coordinates
                            let proj_pos = Vector4f::new(
                                (x as f32 + 0.5) / (width) as f32 * 2. - 1.,
                                (y as f32 + 0.5) / (height) as f32 * 2. -1.,
                                depth,
                                1.);

                            //print!("{};{};{}, ", proj_pos.x, proj_pos.y, proj_pos.z);

                            let mut camera_pos = inv_projection_matrix * proj_pos;

                            camera_pos.x = camera_pos.x / camera_pos.w;
                            camera_pos.y = camera_pos.y / camera_pos.w;
                            camera_pos.z = camera_pos.z / camera_pos.w;

                            //print!("{};{};{};{}, ", camera_pos.x, camera_pos.y, camera_pos.z, camera_pos.w);

                            print!("{}, ", depth);// -camera_pos.z);
                            i = i + 1;
                        }
                        println!("");
                    }
                    println!("");
                }
            }

            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
        }

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

        // draw filled box for debugging
        // let color = vec![1.,1.,0.,1.];
        // let mx_local_to_gl = camera.get_world_to_gl() * Matrix4f::from_scale(4.0);
        // box_drawer.draw_filled(&color, &camera.get_world_to_camera(), &mx_local_to_gl);

        if show_octree_view {
            draw_octree_view(&box_drawer, &camera, &camera_octree, &visible_nodes, &occlusion_world_to_proj_matrices, &mut node_views, &node_drawer);
        }

        window.gl_swap_window();

        let samples_passed = 0;//gl_query.query_samples_passed();

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
