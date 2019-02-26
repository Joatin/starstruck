extern crate glsl_to_spirv;

use std::error::Error;
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<Error>> {
    use glsl_to_spirv::ShaderType;

    // Tell the build script to only run again if we change our source shaders
    println!("cargo:rerun-if-changed=examples/shaders");

    let env_dir = env::var("OUT_DIR")?;
    let out_dir = Path::new(&env_dir);

    dbg!(out_dir);

    // Create destination path if necessary
    // std::fs::create_dir(out_dir.join("/shaders")).expect("Build.rs is not allowed to write to OUT_DIR");
    dbg!(out_dir);

    for entry in std::fs::read_dir("examples/shaders")? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let in_path = entry.path();

            // Support only vertex and fragment shaders currently
            let shader_type =
                in_path
                    .extension()
                    .and_then(|ext| match ext.to_string_lossy().as_ref() {
                        "vert" => Some(ShaderType::Vertex),
                        "frag" => Some(ShaderType::Fragment),
                        _ => None,
                    });

            if let Some(shader_type) = shader_type {
                use std::io::Read;

                let source = std::fs::read_to_string(&in_path)?;
                let mut compiled_file = glsl_to_spirv::compile(&source, shader_type)?;

                let mut compiled_bytes = Vec::new();
                compiled_file.read_to_end(&mut compiled_bytes)?;

                let out_path = out_dir.join(format!(
                    "{}.spv",
                    in_path.file_name().unwrap().to_string_lossy()
                ));
                dbg!(&out_dir);
                dbg!(&out_path);

                let mut f = File::create(&out_path)?;


                f.write_all(&compiled_bytes)?;
            }
        }
    }

    Ok(())
}