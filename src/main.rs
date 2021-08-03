use arcana::camera::FreeCamera3Controller;
use {
    arcana::*,
    rapier3d::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder},
        na,
    },
};

#[derive(Clone, Debug)]
struct Block;

impl Block {
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        struct BlockCollider {
            cuboid: Collider,
        }

        let cuboid = cx
            .res
            .with(|| BlockCollider {
                cuboid: ColliderBuilder::cuboid(0.02, 0.02, 0.002)
                    .friction(0.5)
                    .restitution(0.8)
                    .build(),
            })
            .cuboid
            .clone();

        let physical_data = cx.res.with(PhysicsData3::new);

        let body = physical_data.bodies.insert(
            RigidBodyBuilder::new_dynamic()
                .linvel(na::Vector3::new(
                    rand::random::<f32>() - 0.5,
                    rand::random::<f32>() - 0.5,
                    rand::random::<f32>() - 0.5,
                ))
                .angvel(na::Vector3::new(
                    rand::random::<f32>() - 0.5,
                    rand::random::<f32>() - 0.5,
                    rand::random::<f32>() - 0.5,
                ))
                .build(),
        );

        let collider =
            physical_data
                .colliders
                .insert_with_parent(cuboid, body, &mut physical_data.bodies);


        let entity = cx.world.spawn((
            self,
            graphics::Mesh::from_generator_pos(&genmesh::generators::Cube::new(), sierra::BufferUsage::VERTEX | sierra::BufferUsage::INDEX, cx.graphics, sierra::IndexType::U16) 
                
            ,
            graphics::Material {
                albedo_factor: [0.3.into(), 0.4.into(), 0.5.into()],
                ..Default::default()
            },
            Global3::new(
                na::Translation3::new(
                    rand::random::<f32>() * 1.5 - 0.75,
                    rand::random::<f32>() * 1.5 - 0.75,
                    rand::random::<f32>() * 1.5 - 0.75,
                )
                .into(),
            ),
            body,
        ));

        entity
    }
}

fn main() {
    game3(|mut game: arcana::Game| async move {
        game.scheduler.add_system(camera::FreeCameraSystem);
        //game.control. (
        //    game.viewport.camera(),
        //    camera::FreeCamera3Controller::new(),
        //    &mut game.world,
        //)
        //.unwrap();

        
        
        let controller1 = EntityController::assume_control(
            FreeCamera3Controller::new(),
            10,
            game.viewport.camera(),
            &mut game.world,
        )?;

        game.control.add_global_controller(controller1);

        for _ in 0..10 {
            Block.spawn(game.cx());
        }

        let physical_data = game.res.with(PhysicsData3::new);

        let top = physical_data
            .bodies
            .insert(RigidBodyBuilder::new_static().build());
        let bottom = physical_data
            .bodies
            .insert(RigidBodyBuilder::new_static().build());
        let left = physical_data
            .bodies
            .insert(RigidBodyBuilder::new_static().build());
        let right = physical_data
            .bodies
            .insert(RigidBodyBuilder::new_static().build());

        physical_data.colliders.insert_with_parent(
            ColliderBuilder::halfspace(na::UnitVector3::new_normalize(na::Vector3::new(0.0, 1.0, 0.6)))
                .build(),
            top,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert_with_parent(
            ColliderBuilder::halfspace(na::UnitVector3::new_normalize(na::Vector3::new(0.0, -1.0, 0.6)))
                .build(),
            bottom,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert_with_parent(
            ColliderBuilder::halfspace(na::UnitVector3::new_normalize(na::Vector3::new(1.0, 0.0, 0.6)))
                .build(),
            left,
            &mut physical_data.bodies,
        );
        physical_data.colliders.insert_with_parent(
            ColliderBuilder::halfspace(na::UnitVector3::new_normalize(na::Vector3::new(-1.0, 0.0, 0.6)))
                .build(),
            right,
            &mut physical_data.bodies,
        );

        game.world
            .spawn((top, Global3::new(na::Translation3::new(0.0, -0.8, 0.6).into())));
        game.world
            .spawn((bottom, Global3::new(na::Translation3::new(0.0, 0.8, 0.6).into())));
        game.world
            .spawn((left, Global3::new(na::Translation3::new(-0.8, 0.0, 0.6).into())));
        game.world
            .spawn((right, Global3::new(na::Translation3::new(0.8, 0.0, 0.6).into())));
        game.world
            .spawn((right, Global3::new(na::Translation3::new(0.8, 0.0, -0.6).into())));
        game.world
            .spawn((right, Global3::new(na::Translation3::new(0.8, 0.0, 0.6).into())));

        game.scheduler
            .add_fixed_system(Physics3::new(), TimeSpan::MILLISECOND * 20);

        Ok(game)
    })
}
