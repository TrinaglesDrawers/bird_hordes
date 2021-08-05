mod light;
mod renderer;

use arcana::{
    camera::{FreeCamera, FreeCamera3Controller, FreeCameraSystem},
    game3,
    renderer::Renderer,
    EntityController, Global3, SystemContext,
};
use rapier3d::na;

fn main() {
    game3(|mut game: arcana::Game| async move {
        game.renderer = Some(Box::new(self::renderer::MyRenderer::new(
            &mut game.graphics,
        )?));

        let camera = game.viewport.camera();

        game.scheduler.add_system(FreeCameraSystem);

        let controller1 = EntityController::assume_control(
            FreeCamera3Controller::new(),
            10,
            camera,
            &mut game.world,
        )?;

        game.world
            .insert(camera, (Global3::identity(), FreeCamera::new(50.0)))
            .unwrap();

        game.control.add_global_controller(controller1);

        game.res.insert(light::DirLight {
            dir: na::Vector3::new(0.5, -1.0, -0.5).normalize(),
            color: [0.8, 0.6, 0.4],
        });

        // let mut handle = game
        //     .loader
        //     .load::<assets::object::Object>(
        //         &"18d6f877-88e8-46f9-b65d-ae23a4b70588".parse().unwrap(),
        //     )
        //     .await;

        // let object = handle.get(&mut game.graphics)?;

        // for i in -5..=5 {
        //     for j in -5..=5 {
        //         game.world.spawn((
        //             object.primitives[0].mesh.clone(),
        //             Global3::new(na::Translation3::new(i as f32, 0.0, j as f32).into()),
        //         ));
        //     }
        // }

        let mut tile_set = game
            .loader
            .load::<terragen::TileSet>(&"688ab66a-b154-44db-a361-b26056fcc100".parse().unwrap())
            .await;

        let tile_set = tile_set.get(&mut game.graphics)?.clone();

        let mut terrain = terragen::Terrain::new(
            tile_set,
            1,
            na::Vector3::new(4.0, 2.0, 4.0),
            na::Vector2::new(1, 1),
            rand::random(),
        );

        game.scheduler.add_system(move |mut cx: SystemContext<'_>| {
            let g = *cx.world.query_one_mut::<&Global3>(camera).unwrap();
            terrain.spawn_around(g.iso.translation.vector.into(), 100.0, cx.task());
        });

        Ok(game)
    })
}
