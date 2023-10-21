//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#![warn(
	clippy::cargo,
	clippy::pedantic,
	clippy::nursery,

	clippy::exit,
	clippy::filetype_is_file,
	clippy::float_cmp_const,
	clippy::get_unwrap,
	clippy::integer_division,
	clippy::mem_forget,
	clippy::todo,
	clippy::unimplemented,
	clippy::unreachable,
	clippy::verbose_file_reads,
	clippy::unseparated_literal_suffix,
	clippy::unneeded_field_pattern,
	clippy::suspicious_xor_used_as_pow,
	clippy::string_to_string,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::ref_patterns,
	clippy::rc_mutex,
	clippy::format_push_string,
	clippy::fn_to_numeric_cast_any,
	clippy::dbg_macro
)]

#![allow(
	clippy::cargo_common_metadata,
	clippy::multiple_crate_versions,
	clippy::cast_precision_loss,
	clippy::cast_possible_truncation,
	clippy::cast_sign_loss,
	clippy::cognitive_complexity,
	clippy::too_many_lines,
	clippy::cast_lossless
)]

use std::array::from_fn;
use std::num::NonZeroU32;
use std::time::Instant;

use rayon_macro::parallel;
use softbuffer::{Context, Surface};
use tiny_skia::{Pixmap, PathBuilder, Stroke, Paint, Transform, Color, FillRule, Shader, PixmapMut, Path, BlendMode, PixmapRef, PremultipliedColorU8};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent, ElementState, KeyEvent};
use winit::event_loop::{EventLoop, DeviceEvents};
use winit::keyboard::{Key, NamedKey};
use winit::window::{WindowBuilder, Fullscreen, Theme, Icon};
use ultraviolet::{Lerp, Vec2};

// STEP EVERY EVENT TO MUSIC
// INCREASE BPM TO SPEED UP GAME

struct Particle {
	pos: Vec2,
	life: f32,
	scale: f32,
	opacity: u8
}

struct Grain {
	pos: Vec2,
	life: f32,
	opacity: u8
}

struct Node {
	pos: Vec2,
	connections: Vec<Node>
}

struct Entity {
	node: Node
}

fn circle_fits(pos: Vec2, radius: f32, pixmap: PixmapRef) -> bool {
	let mid = Vec2 { x: pixmap.width() as f32, y: pixmap.height() as f32 } * 0.5;

	(mid.x - pos.x).abs() < mid.x + radius && (mid.y - pos.y).abs() < mid.y + radius
}

fn stroke_circle(pos: Vec2, radius: f32, color: Color, pixmap: &mut PixmapMut) {
	if circle_fits(pos, radius, pixmap.as_ref()) {
		pixmap.stroke_path(
			&PathBuilder::from_circle(pos.x, pos.y, radius).unwrap(),
			&Paint { shader: Shader::SolidColor(color), ..Paint::default() },
			&Stroke::default(),
			Transform::identity(),
			None
		);
	}
}

fn fill_circle(pos: Vec2, radius: f32, color: Color, pixmap: &mut PixmapMut) {
	if circle_fits(pos, radius, pixmap.as_ref()) {
		pixmap.fill_path(
			&PathBuilder::from_circle(pos.x, pos.y, radius).unwrap(),
			&Paint { shader: Shader::SolidColor(color), ..Paint::default() },
			FillRule::default(),
			Transform::identity(),
			None
		);
	}
}

fn stroke_fill_path(
	pixmap: &mut PixmapMut,
	path: &Path,
	stroke_paint: &Paint,
	fill_paint: &Paint,
	stroke: &Stroke
) {
	pixmap.stroke_path(
		path,
		stroke_paint,
		stroke,
		Transform::identity(),
		None
	);

	pixmap.fill_path(
		path,
		fill_paint,
		FillRule::default(),
		Transform::identity(),
		None
	);
}

fn main() {
	let background = Color::from_rgba8(25, 25, 35, 255);

	let event_loop = EventLoop::new().unwrap();
	event_loop.listen_device_events(DeviceEvents::Never);

	let window = {
		let mut icon = Pixmap::new(32, 32).unwrap();

		icon.as_mut().fill(background);

		stroke_fill_path(
			&mut icon.as_mut(),
			&PathBuilder::from_circle(16.0, 16.0, 8.0).unwrap(),
			&Paint {
				shader: Shader::SolidColor(Color::from_rgba8(255, 134, 4, 255)),
				blend_mode: BlendMode::Source,
				..Paint::default()
			},
			&Paint {
				shader: Shader::SolidColor(Color::from_rgba8(35, 35, 55, 125)),
				..Paint::default()
			},
			&Stroke {
				width: 4.0,
				..Default::default()
			}
		);

		let w = icon.width();
		let h = icon.height();

		WindowBuilder::new()
			.with_title("Game")
			.with_inner_size(LogicalSize::new(1280, 720))
			.with_min_inner_size(LogicalSize::new(256, 144))
			.with_theme(Some(Theme::Dark))
			.with_window_icon(Some(Icon::from_rgba(icon.take(), w, h).unwrap()))
			.build(&event_loop)
			.unwrap()
	};

	window.set_cursor_visible(false);

	let mut surface = {
		let context = unsafe { Context::new(&window) }.unwrap();

		unsafe { Surface::new(&context, &window) }.unwrap()
	};

	let size = window.inner_size();
	let mut width_f32 = size.width as f32;
	let mut height_f32 = size.height as f32;
	let mut vec_size = Vec2 { x: width_f32, y: height_f32 };

	let mut particles: [Particle; 1000] = from_fn(|_| Particle {
		pos: Vec2 { x: fastrand::f32() - 0.25, y: fastrand::f32() - 0.25 } * vec_size * 2.0,
		life: fastrand::f32(),
		scale: fastrand::f32(),
		opacity: fastrand::u8(100..255)
	});

	let mut dust: [Grain; 2000] = from_fn(|_| Grain {
		pos: Vec2 { x: fastrand::f32() - 0.25, y: fastrand::f32() - 0.25 } * vec_size * 2.0,
		life: fastrand::f32(),
		opacity: fastrand::u8(35..100)
	});

	let mut entities: Vec<Entity> = vec![Entity {
		node: Node {
			pos: vec_size * 0.5,
			connections: vec![]
		}
	}];

	let mut mpos = Vec2 { x: 0.0, y: 0.0 };
	let mut world = Vec2 { x: 0.0, y: 0.0 };
	let mut click = 0.0;

	let now = Instant::now();
	let mut last_elapsed = now.elapsed().as_secs_f32();

	let mut avg = 0.0_f32;

	event_loop.run(move |event, elwt| { match event {
		Event::AboutToWait => {
			let new_elapsed = now.elapsed().as_secs_f32();
			let delta = new_elapsed - last_elapsed;
			let fps = 1.0 / delta;
			avg = avg.mul_add(29.0, fps) / 30.0;
			println!("{avg}");

			click = click.lerp(0.0, (delta * 5.0).min(1.0));

			let character = &mut entities[0].node;
			character.pos = character.pos.lerp(mpos - world, ((click + 0.1) * delta).min(1.0));

			let mdist_x = width_f32.mul_add(0.5, -mpos.x);
			let mdist_y = height_f32.mul_add(0.5, -mpos.y);
			let mdist = (mdist_x.mul_add(mdist_x, mdist_y * mdist_y) / width_f32.mul_add(width_f32, height_f32 * height_f32)).sqrt();

			world = world.lerp(world + vec_size * 0.5 - (mpos + character.pos + world) * 0.5, (mdist * delta).min(1.0));

			parallel!(for grain in &mut dust {
				if grain.life > 0.0 {
					grain.life -= 0.125 * delta;
				} else {
					grain.pos = Vec2 { x: fastrand::f32() - 0.25, y: fastrand::f32() - 0.25 } * vec_size * 2.0 - world;
					grain.life = 1.0;
					grain.opacity = fastrand::u8(35..100);
				}

				let mdist = click.mul_add(-0.05, 1.0 + ((grain.pos - mpos + world) / vec_size).mag()).powi(16);
				let plr_dist = ((grain.pos - character.pos) / vec_size).mag().mul_add(0.75, 1.0).powi(16);

				let speed = 5.0 * delta;

				let min = Vec2 { x: -0.01, y: -0.01 };
				let max = Vec2 { x: 0.01, y: 0.01 };

				grain.pos += Vec2 { x: fastrand::f32() - 0.5, y: fastrand::f32() - 0.5 } * vec_size / Vec2 { x: 2560.0, y: 1440.0 } + vec_size * speed * (
					(grain.pos - mpos + world).clamped(min, max) / mdist * (click + 1.0) +
					(grain.pos - character.pos).clamped(min, max) / plr_dist
				);
			});

			parallel!(for particle in &mut particles {
				if particle.life > 0.0 {
					particle.life -= 0.125 * delta;
				} else {
					particle.pos = Vec2 { x: fastrand::f32() - 0.25, y: fastrand::f32() - 0.25 } * vec_size * 2.0 - world;
					particle.life = 1.0;
					particle.scale = fastrand::f32();
					particle.opacity = fastrand::u8(100..255);
				}

				let mdist = click.mul_add(-0.05, ((particle.pos - mpos + world) / vec_size).mag().mul_add(1.5, 1.0)).powi(16);
				let plr_dist = ((particle.pos - character.pos) / vec_size).mag().mul_add(0.75, 1.0).powi(16);

				let speed = 15.0 * delta;

				let min = Vec2 { x: -0.01, y: -0.01 };
				let max = Vec2 { x: 0.01, y: 0.01 };

				particle.pos += Vec2 { x: fastrand::f32() - 0.5, y: fastrand::f32() - 0.5 } * vec_size / Vec2 { x: 2560.0, y: 1440.0 } + vec_size * speed * (
					(particle.pos - mpos + world).clamped(min, max) / mdist * (click + 1.0) +
					(particle.pos - character.pos).clamped(min, max) / plr_dist
				);
			});

			window.request_redraw();
			last_elapsed = new_elapsed;
		},
		Event::WindowEvent { event, .. } => match event {
			WindowEvent::RedrawRequested => {
				let mut buffer = surface.buffer_mut().unwrap();

				let mut pixmap = PixmapMut::from_bytes(
					bytemuck::cast_slice_mut(&mut buffer),
					width_f32 as u32,
					height_f32 as u32
				).unwrap();

				pixmap.fill(background);

				for grain in &mut dust {
					let radius = width_f32.min(height_f32) * 0.005 * (1.0 - grain.life);
	
					fill_circle(grain.pos + world, radius, Color::from_rgba8(173, 216, 230, (grain.opacity as f32 * grain.life) as u8), &mut pixmap);
				}
	
				for particle in &mut particles {
					let radius = width_f32.min(height_f32) * 0.025 * (1.0 - particle.life) * particle.scale;
	
					stroke_circle(particle.pos + world, radius, Color::from_rgba8(173, 216, 230, (particle.opacity as f32 * particle.life) as u8), &mut pixmap);
				}

				for entity in &entities {
					let mut nodes = vec![&entity.node];
					for node in &entity.node.connections {
						nodes.push(node);
					}
	
					for node in nodes {
						stroke_circle(node.pos + world, width_f32.min(height_f32) * 0.05, Color::from_rgba8(173, 216, 230, 200), &mut pixmap);
					}
				}
	
				let mouse_size = width_f32.min(height_f32).mul_add(0.05, -click * 25.0).max(1.0);
				stroke_circle(mpos, mouse_size, Color::from_rgba8(173, 216, 230, 255), &mut pixmap);

				parallel!(for pix in pixmap.pixels_mut() {
					*pix = PremultipliedColorU8::from_rgba(pix.blue(), pix.green(), pix.red(), u8::MAX).unwrap();
				});

				window.pre_present_notify();
				buffer.present().unwrap();
			},
			WindowEvent::MouseInput { state: ElementState::Pressed, .. } => click = 1.0,
			WindowEvent::CursorMoved { position, .. } => mpos = Vec2 { x: position.x as f32, y: position.y as f32 },
			WindowEvent::KeyboardInput {
				event: KeyEvent {
					logical_key: Key::Named(NamedKey::F11),
					state: ElementState::Pressed,
					repeat: false,
					..
				},
				..
			} => window.set_fullscreen(
				if window.fullscreen().is_none() {
					Some(Fullscreen::Borderless(None))
				} else {
					None
				}
			),
			WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
				width_f32 = size.width as f32;
				height_f32 = size.height as f32;
				vec_size = Vec2 { x: width_f32, y: height_f32 };

				surface.resize(
					NonZeroU32::new(size.width).unwrap(),
					NonZeroU32::new(size.height).unwrap(),
				).unwrap();
			},
			WindowEvent::CloseRequested => elwt.exit(),
			_ => ()
		},
		_ => ()
	}}).unwrap();
}
