use winit::{
	window::Window,
	window::WindowId
};

use std::fs;

use rand::prelude::*;
use crate::game_object::*;
use crate::game_renderer::*;
use crate::game_input::*;
use cgmath::Vector3;
use cgmath::InnerSpace;

const SKY_Z:f32 = 0.0;
const SUN_Z:f32 = 15.0;
const CLOUD_Z:f32 = 20.0;
const HILL_Z:f32 = 30.0;
const BUILDING_Z:f32 = 50.0;
const CHARACTER_Z:f32 = 100.0;

#[allow(dead_code)] 
trait GameAsset {
    fn asset_name(&self) -> &String;
}

struct GameTexture {
	name: String,
}

impl GameAsset for GameTexture {
     fn asset_name(&self) -> &String {
		 return &self.name;
	 }
}

#[allow(dead_code)] 
#[derive(Default)]
pub struct AssetManager {
	resources: Vec<Box<dyn GameAsset>>,
}

#[allow(dead_code)] 
impl AssetManager {
	pub fn new() -> Self {
		Self {
			..Default::default()
		}
	}
	fn load_asset(_asset_name: String) {

	}
}

#[allow(dead_code)] 
pub struct GameEngine<'a> {
	pub input_manager: InputManager,
	asset_manager: AssetManager,
	renderer: Renderer<'a>,
	window_id: WindowId,
	game_objects: Vec<GameObject>,
	game_start_time:  std::time::Instant,
	current_frame_time:  std::time::Instant,
	next_enemy_spawn_time: f32,
	num_enemies: u32,

	// data
	enemy_spawn_timer: f32,
	enemy_speed: f32,
}

impl<'a> GameEngine<'a> {
    pub async fn new(window: Window) -> Self {
		let input_manager = InputManager::new();
        let asset_manager = AssetManager::new();

		// Load config file
		let config_file_text = fs::read_to_string("GameAssets/game_config.txt").expect("Missing config files!");
		let json_file = json::parse(&config_file_text).unwrap();
		
		let json_val = json_file["enemy_spawn_timer"].as_f32();
		let mut enemy_spawn_timer = 0.01;
		match json_val {
			Some(val) => { enemy_spawn_timer = val; }
			None => ()
		}

		let json_val = json_file["enemy_speed"].as_f32();
		let mut enemy_speed = 0.01;
		match json_val {
			Some(val) => { enemy_speed = val;}
			None => ()
		}

		let mut max_instances = 10000;
		let json_val = json_file["max_instances"].as_usize();
		match json_val {
			Some(val) => { max_instances = val; }
			None => ()
		}

		let mut graphics_back_end = "default";
		let json_val = json_file["graphics_back_end"].as_str();
		match json_val {
			Some(val) => {
				graphics_back_end = val;
			}
			None => ()
		}

		let mut power_pref = "default";
		let json_val = json_file["graphics_power_pref"].as_str();
		match json_val {
			Some(val) => {
				power_pref = val;
			}
			None => ()

		}
		let window_id = window.id();
		let renderer = Renderer::new(window, graphics_back_end, power_pref, max_instances).await;
		let cur_time = std::time::Instant::now();

		Self {
			input_manager,
			asset_manager,
			renderer,
			window_id,
			game_objects: Vec::<GameObject>::new(),
			game_start_time:  cur_time,
			current_frame_time : cur_time,
			next_enemy_spawn_time: cur_time.elapsed().as_secs_f32() + enemy_spawn_timer,
			num_enemies: 0,
			enemy_spawn_timer,
			enemy_speed,
		}
    }
	
	pub fn window_id(&self) -> WindowId {
		self.window_id
	}
	
	pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		self.renderer.resize(new_size);
	}

	pub fn update_enemies(&mut self) {

		if self.game_objects.len() >= self.renderer.max_instances {
			return
		}

		let game_time = self.game_start_time.elapsed().as_secs_f32();
		if game_time > self.next_enemy_spawn_time {

			self.next_enemy_spawn_time  = game_time + self.enemy_spawn_timer;
			self.num_enemies = self.num_enemies + 1;
			
			let mut start_x = 1.0;
			let mut vel_x = -self.enemy_speed;

			let randnum = rand::thread_rng().gen_range(1..=2);
		    if randnum == 2 {
				start_x = start_x * -1.0;
				vel_x = vel_x * -1.0;
			}
			let y_pos:f32  = rand::thread_rng().gen_range(0.0..=0.75);

			// Create Enemy
			self.game_objects.push(GameObject { 
				position: (start_x, y_pos, CHARACTER_Z).into(),
				scale: (0.1, 0.15, 0.15).into(),
				direction: (1.0, 0.0, 0.0).into(),
				velocity: (vel_x, 0.0, 0.0).into(),
				object_type: GameObjectType::Robot,
				object_state: GameObjectState::Running,
				next_attack_time: 0.0,
				texture_index: 0,
				sprite_index: 8,
				anim_frame: 0,
				life_start_time: std::time::Instant::now(),
				state_start_time: std::time::Instant::now(),
				gravity_scale: 0.0,
				is_enemy: true
			});
		}
	}

	pub fn update_projectiles(&mut self) {

		let mut i = 0;
		while i < self.game_objects.len() {
			if matches!(self.game_objects[i].object_type, GameObjectType::Projectile) == false {
				i = i + 1;
				continue;
			}

			let mut j = 0;
			while j < self.game_objects.len() {

				// Don't hit other projectiles
				if i == j || matches!(self.game_objects[j].object_type, GameObjectType::Projectile) {
					j = j + 1;
					continue;
				}

				// Allegiance test
				if self.game_objects[i].is_enemy == self.game_objects[j].is_enemy {
					j = j + 1;
					continue;
				}

				let dist = cgmath::Vector2::<f32>::new(self.game_objects[i].position.x - self.game_objects[j].position.x, self.game_objects[i].position.y - self.game_objects[j].position.y).magnitude2();
				if dist < 0.05 {
					if i > j {
						self.game_objects.remove(i);
						self.game_objects.remove(j);
					} else {
						self.game_objects.remove(j);
						self.game_objects.remove(i);
					}
					break;
				}
				j = j + 1;

			}

			i = i + 1;
		}
	}

	pub fn tick_frame(&mut self) {
		let _delta_time_secs = self.current_frame_time.elapsed().as_secs_f32();
        self.current_frame_time = std::time::Instant::now();

		// Player Movement
        let mut move_vec:cgmath::Vector3<f32> = (0.0, 0.0, 0.0).into();

        if self.input_manager.left_pressed() {
            move_vec = Vector3::new(-1.0, 0.0, 0.0);
			self.game_objects[0].direction.x = -1.0;
        }

        if self.input_manager.right_pressed() {
           move_vec = Vector3::new(1.0, 0.0, 0.0);
		   self.game_objects[0].direction.x = 1.0;
		}

        if self.input_manager.up_pressed {
            move_vec.y = 1.0;
        }

        self.game_objects[0].set_velocity(move_vec);

		self.update_enemies();
		self.update_projectiles();

		// Player Action
		if self.input_manager.fire_pressed() && self.game_objects[0].start_attack() {
			let direction = self.game_objects[0].direction;
			let velocity = if direction.x > 0.0 { (5.0, 0.0, 0.0).into() } else { (-5.0, 0.0, 0.0).into() };
			let new_projectile = GameObject { 
				position: self.game_objects[0].position + direction * 0.1,
				scale: (0.035, 0.05, 0.05).into(),
				direction,
				velocity,
				object_type: GameObjectType::Projectile,
				object_state: GameObjectState::Idle,
				next_attack_time: 0.0,
				texture_index: 0,
				sprite_index: 5,
				anim_frame: 0,
				life_start_time: std::time::Instant::now(),
				state_start_time: std::time::Instant::now(),
				gravity_scale: 0.0,
				is_enemy: false
			};

			self.game_objects.push(new_projectile);
		}

		// Update game objects
		let game_object_iter = self.game_objects.iter_mut();
		for game_object in game_object_iter {
			game_object.update(_delta_time_secs);
		}

		self.render_frame();
//		self.renderer.window().request_redraw();
	}

	pub fn render_frame(&mut self) -> bool {
		
		let render_result = self.renderer.render(&self.game_objects, self.game_start_time.elapsed().as_secs_f32());
		match render_result {
			Ok(_) => {}
			Err(wgpu::SurfaceError::Lost) => self.renderer.resize(self.renderer.size),
			Err(wgpu::SurfaceError::OutOfMemory) => { return false }
			Err(e) => eprintln!("{:?}", e),
		}

		true
	}

	pub fn initialize_world(&mut self)
	{
		// Create Player
		self.game_objects.push(GameObject { 
			position: (0.0, 0.0, CHARACTER_Z).into(),
			scale: (0.1, 0.15, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Character,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 0,
			sprite_index: 0,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 1.1,
			is_enemy: false
		});


		// Sky
		self.game_objects.push(GameObject { 
			position: (0.0, 0.0, SKY_Z).into(),
			scale: (2.0, 2.0, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 25,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		// Sun
		self.game_objects.push(GameObject { 
			position: (0.0, 1.0, SUN_Z).into(),
			scale: (0.1, 0.15, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 20,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		// Clouds
		let mut i = 0;
		while i < 10 {

			let rand_x = rand::thread_rng().gen_range(-1.0..=1.0);
			let rand_y = rand::thread_rng().gen_range(0.8..=1.1);
			let x_speed = rand::thread_rng().gen_range(0.05..=0.1);
			let x_speed = if rand::thread_rng().gen_range(0..=1) == 1 { -x_speed } else { x_speed };

			// Cloud
			self.game_objects.push(GameObject { 
				position: (rand_x,rand_y, CLOUD_Z).into(),
				scale: (0.1, 0.15, 0.15).into(),
				direction: (1.0, 0.0, 0.0).into(),
				velocity: (0.0, 0.0, 0.0).into(),
				object_type: GameObjectType::Cloud,
				object_state: GameObjectState::Idle,
				next_attack_time: 0.0,
				texture_index: 1,
				sprite_index: 18 + rand::thread_rng().gen_range(0..=1),
				anim_frame: 0,
				life_start_time: std::time::Instant::now(),
				state_start_time: std::time::Instant::now(),
				gravity_scale: 0.0,
				is_enemy: false
			});

			match self.game_objects.last_mut() {
				Some(game_obj) => {
					game_obj.set_velocity(Vector3::<f32>::new(x_speed, 0.0, 0.0));
				}

				None => ()
			
			}
			i = i + 1;
		}

		// Hills
		self.game_objects.push(GameObject { 
			position: (0.0, 0.75, HILL_Z).into(),
			scale: (1.0, 1.6, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 21,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		// Buildings
		self.game_objects.push(GameObject { 
			position: (-0.8, 0.4, BUILDING_Z).into(),
			scale: (0.1, 0.4, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 16,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (-0.6, 0.2, BUILDING_Z).into(),
			scale: (0.1, 0.2, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 16,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (-0.35, 0.3, BUILDING_Z).into(),
			scale: (0.13, 0.3, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 17,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		
		self.game_objects.push(GameObject { 
			position: (-0.18, 0.5, BUILDING_Z).into(),
			scale: (0.1, 0.5, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 16,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (0.0, 0.1, BUILDING_Z).into(),
			scale: (0.11, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 17,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (0.2, 0.3, BUILDING_Z).into(),
			scale: (0.1, 0.3, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 17,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (0.4, 0.2, BUILDING_Z).into(),
			scale: (0.13, 0.2, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 16,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (0.65, 0.3, BUILDING_Z).into(),
			scale: (0.09, 0.3, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 16,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (0.85, 0.7, BUILDING_Z).into(),
			scale: (0.12, 0.7, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 16,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		// Trees
		self.game_objects.push(GameObject { 
			position: (-0.95, 0.1, BUILDING_Z + 1.0).into(),
			scale: (0.07, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 23,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (-0.6, 0.1, BUILDING_Z + 1.0).into(),
			scale: (0.07, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 23,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (-0.3, 0.1, BUILDING_Z + 1.0).into(),
			scale: (0.07, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 24,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});

		self.game_objects.push(GameObject { 
			position: (0.05, 0.1, BUILDING_Z + 1.0).into(),
			scale: (0.07, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 24,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});
		self.game_objects.push(GameObject { 
			position: (0.15, 0.1, BUILDING_Z + 1.0).into(),
			scale: (0.07, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 24,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});
		self.game_objects.push(GameObject { 
			position: (0.55, 0.1, BUILDING_Z + 1.0).into(),
			scale: (0.07, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 23,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});
		self.game_objects.push(GameObject { 
			position: (0.85, 0.1, BUILDING_Z + 1.0).into(),
			scale: (0.07, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 23,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});
		self.game_objects.push(GameObject { 
			position: (0.95, 0.1, BUILDING_Z + 1.0).into(),
			scale: (0.07, 0.1, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 24,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});
		self.game_objects.push(GameObject { 
			position: (0.5, -0.5, BUILDING_Z + 2.0).into(),
			scale: (0.5, 0.5, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 22,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});
		self.game_objects.push(GameObject { 
			position: (-0.5, -0.5, BUILDING_Z + 2.0).into(),
			scale: (0.5, 0.5, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 22,
			anim_frame: 0,
			life_start_time: std::time::Instant::now(),
			state_start_time: std::time::Instant::now(),
			gravity_scale: 0.0,
			is_enemy: false
		});
	}
}