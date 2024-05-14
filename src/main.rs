use cgmath::Vector3;

use kb_engine3::log;
use kb_engine3::{
    kb_config::KbConfig,
    kb_engine::KbGameEngine,
    kb_game_object::{GameObject, GameObjectType},
    kb_input::KbInputManager,
    kb_renderer::KbRenderer,
};

const SKY_Z: f32 = 0.0;
const SUN_Z: f32 = 15.0;
const HILL_Z: f32 = 30.0;
const BUILDING_Z: f32 = 50.0;

// Game clients should create a game struct that implement the KbGameEngine trait which provides functions for
// init, update, and collecting game objects for rendering.

pub struct EmptyGame {
    pub game_objects: Vec<GameObject>, // List of objects to be rendered
}

impl KbGameEngine for EmptyGame {
    fn new(_game_config: &KbConfig) -> Self {
        log!("EmptyGame::new() caled...");

        Self {
            game_objects: Vec::<GameObject>::new(),
        }
    }

    async fn initialize_world(&mut self, _game_renderer: &mut KbRenderer<'_>) {
        log!("EmptyGame::initialize_world() caled...");

        let right_vec: Vector3<f32> = Vector3::new(1.0, 0.0, 0.0);

        // Create game objects
        let sky = GameObject::new(
            GameObjectType::Skybox,
            25,
            (0.0, 0.0, SKY_Z).into(),
            right_vec,
            (2.0, 2.0, 1.0).into(),
        );
        self.game_objects.push(sky);

        let sun = GameObject::new(
            GameObjectType::Skybox,
            27,
            (-0.5, 1.0, SUN_Z).into(),
            right_vec,
            (0.15, 0.15, 0.15).into(),
        );
        self.game_objects.push(sun);

        let hill = GameObject::new(
            GameObjectType::Background,
            21,
            (0.0, 0.75, HILL_Z).into(),
            right_vec,
            (2.0, 1.6, 0.15).into(),
        );
        self.game_objects.push(hill);

        let left_road = GameObject::new(
            GameObjectType::Background,
            22,
            (1.0, -0.5, BUILDING_Z + 2.0).into(),
            right_vec,
            (1.0, 0.5, 1.0).into(),
        );
        self.game_objects.push(left_road);

        let right_road = GameObject::new(
            GameObjectType::Background,
            22,
            (-1.0, -0.5, BUILDING_Z + 2.0).into(),
            right_vec,
            (1.0, 0.5, 1.0).into(),
        );
        self.game_objects.push(right_road);
    }

    fn get_game_objects(&self) -> &Vec<GameObject> {
        &self.game_objects
    }

    fn tick_frame_internal(
        &mut self,
        _game_renderer: &mut KbRenderer,
        _input_manager: &KbInputManager,
        _game_config: &KbConfig,
    ) {
        // Add game update logic here
    }
}

fn main() {
    let config_file_text = include_str!("../engine_assets/game_config.txt");
    let game_config = KbConfig::new(config_file_text);

    // Pass your Game's type to run_game() which will create an instance and call functions on it
    let run_game = kb_engine3::run_game::<EmptyGame>(game_config);

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(run_game);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run_game);
    }
}
