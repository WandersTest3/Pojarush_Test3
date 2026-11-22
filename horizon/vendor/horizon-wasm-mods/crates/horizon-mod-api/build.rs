use std::io::Read;

fn main() {
    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let root = manifest.join("../..").canonicalize().unwrap();
    let root = root.to_str().unwrap().replace('\\', "/");
    let dir = manifest.join("pregenerated");
    println!("cargo:rerun-if-changed=pregenerated");
    println!("cargo::rustc-check-cfg=cfg(engine_slots_point_id)");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".gz")) else {
            continue;
        };
        let mut text = String::new();
        flate2::read::GzDecoder::new(std::fs::File::open(&path).unwrap())
            .read_to_string(&mut text)
            .unwrap();
        let text = text.replace("@ROOT@", &root);
        let target = out.join(name);
        if std::fs::read_to_string(&target).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(target, text).unwrap();
        }
    }
}
