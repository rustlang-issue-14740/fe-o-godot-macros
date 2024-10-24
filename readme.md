
> [!WARNING]  
> This requires the new WIP feature [multiple impl blocks](https://github.com/godot-rust/gdext/pull/927) which is why this package references my github branch.

# Helper macros for inter-node communication with Godot Rust 4.x GDExt

Two macros, one gets applied to a trait definition, the other to the impl block of the godot class implementing the trait

```rust
#[fe_o_godot_macros::wrap_trait]
trait MyTrait {
    fn foo();
}

// -------------------------------

#[fe_o_godot_macros::godot_virtual_dispatch]
impl MyTrait for MyCustomNodeClass {
    fn foo(&mut self) {
        bar();
    }
}
```

With the help of these two macros, I can dynamically query and invoke this trait on a ``Gd<Node>``.

```rust

let node : Gd<Node> = // ...

let t : Option<Box<dyn MyTrait>> = try_get_trait_MyTrait(node);

```

## Example

you may have organized the functionality of your nodes into some traits ...

```rust
use godot::prelude::*;
use godot::{classes::Node, obj::Gd};


// --------------
// Trait Hittable
// --------------

pub struct DamageSource {
    pub amount: u32
}

pub struct DamageInfo {
    pub by_actor: Gd<Node>,
    pub by_source: DamageSource
}

#[fe_o_godot_macros::wrap_trait]
pub trait Hittable {
    fn hit(&mut self, dmg: DamageInfo);
}

// -------------
// Trait Healthy
// -------------

pub struct Health {
    pub current_health : u32,
    pub max_health : u32
}

#[fe_o_godot_macros::wrap_trait]
pub trait Healthy {
    fn get_health(&mut self) -> Health;
}
```

You may implement these traits in a Godot Node

```rust
#[derive(GodotClass)]
#[class(base=Area2D, init, tool)]
pub struct Player {

    // ...

    #[var(get, set)]
    #[export]
    max_health: u32,

    #[var(get, set)]
    #[export]
    current_health: u32,
    
    base: Base<Area2D>
}

#[fe_o_godot_macros::godot_virtual_dispatch]
impl Hittable for Player {
    fn hit(&mut self, dmg: crate::traits::DamageInfo) {
        self.current_health = self.current_health.saturating_sub(dmg.by_source.amount);
        if self.current_health == 0 {
            self.set_state(PlayerState::Dying);
        }
    }
}


#[fe_o_godot_macros::godot_virtual_dispatch]
impl Healthy for Player {
    fn get_health(&mut self) -> Health {
        return Health { current_health: self.current_health, max_health: self.max_health };
    }
}
```

but it's hard to impossible to properly expose and reference this functionality across Godot Nodes.

This library helps you with that by providing virtual trait dispatch through Godot Node References.

## Usage

### Example 1: identifying and interacting with the target of a raycast

```rust

// let's say you have a gun node, and that gun gets triggered.
// you might do a raycast and see what you've hit

#[derive(GodotClass)]
#[class(base=Node2D, init)]
pub struct Gun {
    // ...
}

#[godot_api]
impl INode2D for Gun {

    fn process(&mut self, delta: f64) {

        let input = Input::singleton();
        
        let gun_pos : Vector2 = self.bind().get_global_position();
        let mouse_pos : Vector2 = self.bind().get_global_mouse_position();

        let direction = (mouse_pos - gun_pos).normalized();
        let mut angle = godot::global::acos(Vector2 { x: 1.0, y: 0.0 }.dot(direction) as f64);
        if direction.y < 0.0 { angle = -angle };

        if input.is_mouse_button_pressed(MouseButton::LEFT) {
            let mut world = self.bind_mut().get_world_2d().unwrap();
            let mut space = world.get_direct_space_state().unwrap();
            let mut query_ex = PhysicsRayQueryParameters2D::create_ex(gun_pos, direction * 10000.0);
            query_ex = query_ex.collision_mask(0b0010);
            let mut query = query_ex.done().unwrap();
            query.set_collide_with_areas(true);
            let result = space.intersect_ray(query);

            if let Some(collider) = result.get("collider") {
                let possibly_gd: Result<Gd<Node2D>, ConvertError> = Gd::try_from_variant(&collider);

                if let Ok(node) = possibly_gd {

                    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                    // Interesting part below here !

                    if let Some(mut hittable) = try_get_trait_Hittable(node.clone()) {
                        hittable.hit(DamageInfo {by_actor: self.to_gd(), by_source: DamageSource { amount: 50 }});
                    }
                } 
            }
        }
    }
}
```

The interesting code is in the last two lines. We have an arbitrary ``Gd<Node2D>`` and can interrogate it whether it implements a specific trait and get a reference to that trait (``Hittable`` in this case).


### Example 2: implementing a generic health bar that can be attached to an arbitrary node


Similar to the example above, you might have a generic health bar, that can be attached to any Node that implements a trait

```rust

use godot::prelude::*;
use crate::traits::try_get_trait_Healthy;

#[derive(GodotClass)]
#[class(base=Node2D, init, tool)] // tool is required for 'get_configuration_warnings'
pub struct HealthBar {
    #[export]
    owner: Option<Gd<Node2D>>,

    #[var(get, set = set_filled)]
    #[export(range = (0.0, 1.0))]
    filled: f32,

    base: Base<Node2D>
}

#[godot_api]
impl HealthBar {
    
    #[func]
    pub fn set_filled(&mut self, new_value: f32) {
        if self.filled != new_value {
            self.base_mut().queue_redraw();
            self.filled = new_value;
        }
    }
}

#[godot_api]
impl INode2D for HealthBar {

    fn ready(&mut self) {
        if let Some(o) = &self.owner {
            if try_get_trait_Healthy(o.clone()).is_none() {
                godot_error!("Owner must implement the trait 'Healthy'");
            }
        } else {
            godot_error!("Owner should have been set!");
        }
    }

    fn process(&mut self, _delta: f64) {
        if let Some(o) = &self.owner {

            // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!
            // Interesting part below here !

            if let Some(mut healthy) = try_get_trait_Healthy(o.clone()) {
                let health = healthy.get_health();

                self.set_filled(health.current_health as f32 / health.max_health as f32);
            }
        }
    }

    fn draw(&mut self) {
        let filled = self.filled;
        let mut b = self.base_mut();

        let points = vec![
            Vector2 { x: 0.0, y: 0.0 },
            Vector2 { x: 100.0, y: 0.0 },
            Vector2 { x: 110.0, y: 10.0 },
            Vector2 { x: 10.0, y: 10.0 },
            Vector2 { x: 0.0, y: 0.0 },
        ];

        b.draw_polyline(&points.into(), Color::GREEN_YELLOW);

        let x_width = 92.0; // 100%
        let points2 = vec![
            Vector2 { x: 6.0, y: 2.0 },
            Vector2 { x: 6.0 + (filled * x_width), y: 2.0 },
            Vector2 { x: 12.0 + (filled * x_width), y: 8.0 },
            Vector2 { x: 12.0, y: 8.0 },
            Vector2 { x: 6.0, y: 2.0 },
        ];

        b.draw_colored_polygon(&points2.into(), Color::GREEN);

    }

    fn get_configuration_warnings(&self) -> PackedStringArray {

        let mut warnings: Vec<GString> = vec!{};

        if self.owner == None {
            warnings.push("Owner is not set".into());
        }

        return PackedStringArray::from(warnings);
    }
}
```

I have included the complete code, as reference. The relevant part is just the function 'process'


## What does it actually do?

Behind the scenes, the macro that gets applied to the trait defines a wrapper struct and an accessor function

```rust
#[derive(GodotClass)]
#[class]
pub struct Hittable_Wrapper {
    pub other: Box<dyn Hittable>
}

impl GodotDefault for HittableWrapper {

}

impl Hittable for HittableWrapper {
    fn hit(&mut self, dmg: DamageInfo) {
        self.other.hit(dmg);
    }
}

impl<T: godot::prelude::GodotClass> Hittable for Gd<T>
where T : Hittable,
      T : GodotClass + godot::obj::Bounds<Declarer = godot::obj::bounds::DeclUser>,
{
    fn hit(&mut self, dmg: DamageInfo) {
        self.bind_mut().hit(dmg);
    }
}


pub fn try_get_trait_Hittable<T>(node: Gd<T>) -> Option<Box<dyn Hittable>>
where T : Inherits<Node> {
    let mut node: Gd<Node> = node.upcast();
    if node.has_method("get_trait_Hittable".into()) {
        let method_result = node.call("get_trait_Hittable".into(), &[]);
        let enemy : Gd<HittableWrapper> = method_result.to::<Gd<HittableWrapper>>();

        let enemy_box : Box<dyn Hittable> = Box::new(enemy);
        return Some(enemy_box);
    }

    return None;
}
```

The macro that gets applied to the godot node trait implementation defines a godot function that wraps itself

```rust
#[godot_api]
impl Player {
    #[func]
    pub fn get_trait_Hittable(&mut self) -> Gd<HittableWrapper> {
        let wrapped = Hittable_Wrapper { other: Box::new(self.to_gd()) };
        return Gd::from_object(wrapped);
    }
}
```

## Status and what doesn't work

This is quite early and still in a rough state. It requires a fork of gdext to support multiple impl blocks (https://github.com/godot-rust/gdext/pull/927).  
The macro generates a few warnings which I haven't yet bothered to supress.  
It currently assumes all ``fn``s have a mutable self.  
It breaks hot reloading.  
Etc...  

But ... it works!

