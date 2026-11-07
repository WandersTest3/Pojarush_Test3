use horizon_shader_api::manifest::Manifest;

const NEWER: &str = r#"
schema = 1
id = "daybreak"
name = "Daybreak"
version = "0.1.1"
api = "0.1"

[sky]
file = "sky.wgsl"
clouds = false

[material]
file = "material.wgsl"
files = ["lib/light.wgsl"]

[[lights]]
blocks = ["minecraft:torch"]
color = [1.0, 0.8, 0.5]

[shadow]
resolution = 1024
"#;

#[test]
fn a_pack_for_a_newer_horizon_loads_what_this_one_draws() {
    let m = Manifest::parse(NEWER).unwrap();
    assert_eq!(m.unsupported, vec!["lights".to_owned(), "shadow".to_owned()]);
    assert_eq!(m.material.as_ref().unwrap().files, vec!["lib/light.wgsl".to_owned()]);
    assert!(m.files().contains(&"lib/light.wgsl"));
}

#[test]
fn a_misspelled_section_is_still_an_error() {
    let text = NEWER.replace("[shadow]", "[shadows]");
    assert!(Manifest::parse(&text).is_err());
}
