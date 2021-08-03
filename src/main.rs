use arcana::{anim::graph, camera::FreeCamera3Controller};
use {
    arcana::*,
    rapier3d::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder},
        na,
    },
};

fn main() {
    game3(|mut game: arcana::Game| async move {
        game.renderer = Some(Box::new(renderer::vcolor::VcolorRenderer::new(
            &mut game.graphics,
        )?));

        game.scheduler.add_system(camera::FreeCameraSystem);

        let controller1 = EntityController::assume_control(
            FreeCamera3Controller::new(),
            10,
            game.viewport.camera(),
            &mut game.world,
        )?;

        game.world
            .insert(
                game.viewport.camera(),
                (Global3::identity(), camera::FreeCamera::new()),
            )
            .unwrap();

        game.control.add_global_controller(controller1);

        let mut handle = game
            .loader
            .load::<assets::object::Object>(
                &"18d6f877-88e8-46f9-b65d-ae23a4b70588".parse().unwrap(),
            )
            .await;
        let object = handle.get(&mut game.graphics)?;

        for i in -50..=50 {
            for j in -50..=50 {
                game.world.spawn((
                    object.primitives[0].mesh.clone(),
                    Global3::new(na::Translation3::new(i as f32, 0.0, j as f32).into()),
                ));
            }
        }

        Ok(game)
    })
}
