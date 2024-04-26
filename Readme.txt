Hi and thanks for checking out my project!  This project demonstrates using Rust and wgpu 0.19.3 (latest at the time of this project) to build a simple sprite renderer.  

All sprites are drawn with a single draw_indexed call using instancing.

========== Building and Debugging ==========

To run the examples, open a command prompt and cd to the directory this read me is in:
    To run 2D demo #1 enter: kbEngine3\Cargo run
    To run 2D demo #2 enter: cargo run --manifest-path Examples/2D/Cargo.toml
    To run 3D demo #3 enter: cargo run --manifest-path Examples/3D/Cargo.toml

Note:
Be sure to set your working directory to the root of the kbEngine3 folder when debugging in Visual Studio, running from RenderDoc, etc.


========== Config file ==========
Each example uses a config file that lets you control several parameters.  There is an example config file at GameAssets\game_config.txt :

{
    "enemy_spawn_delay": 0.3,
    "enemy_move_speed": 1.0,
    "max_instances": 2000,

    "window_width": 1920,
    "window_height": 1080,

    "_comment": "Valid values 'graphics_back_end' are default vulkan or dx12",
    "graphics_back_end": "default",

    "_comment2": "Valid values for 'graphics_power_pref' are default, low, and high",
    "graphics_power_pref": "default",

    "_comment3": "Valid values for 'vsync' are true and false",
    "vsync": true
}


========== Resources ==========
https://github.com/bennywilson/kbEngine3

https://doc.rust-lang.org/book/ch01-00-getting-started.html

https://sotrh.github.io/learn-wgpu/#what-is-wgpu

https://registry.khronos.org/vulkan/specs/1.3/html/vkspec.html

https://github.com/wolfpld/tracy

Benny Wilson
bennywilson@benny-wilson.com