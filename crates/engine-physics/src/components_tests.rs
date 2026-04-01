use super::{ColliderShape3D, PhysicsMaterial, RigidBody3DBundle, RigidBodyType};

#[test]
fn rigid_body_type_defaults_to_dynamic() {
    assert_eq!(RigidBodyType::default(), RigidBodyType::Dynamic);
}

#[test]
fn physics_material_defaults_match_epic_spec() {
    let material = PhysicsMaterial::default();

    assert_eq!(material.restitution, 0.3);
    assert_eq!(material.friction, 0.7);
    assert_eq!(material.density, 1.0);
}

#[test]
fn default_bundle_uses_box_shape() {
    let bundle = RigidBody3DBundle::default();

    assert!(matches!(bundle.shape, ColliderShape3D::Box { .. }));
}
