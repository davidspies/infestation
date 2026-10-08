use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Fonts (SIL Open Font License) from the Google Fonts repository.
const FONTS: &[(&str, &str)] = &[
    (
        "../assets/fonts/LilitaOne-Regular.ttf",
        "https://github.com/google/fonts/raw/main/ofl/lilitaone/LilitaOne-Regular.ttf",
    ),
    (
        "../assets/fonts/AlegreyaSans-Regular.ttf",
        "https://github.com/google/fonts/raw/main/ofl/alegreyasans/AlegreyaSans-Regular.ttf",
    ),
];

const MINIQUAD_JS_FILES: &[(&str, &str)] = &[
    (
        "../js/gl.js",
        "https://raw.githubusercontent.com/not-fl3/miniquad/master/js/gl.js",
    ),
    (
        "../js/sapp_jsutils.js",
        "https://raw.githubusercontent.com/not-fl3/sapp-jsutils/master/js/sapp_jsutils.js",
    ),
    (
        "../js/quad-url.js",
        "https://raw.githubusercontent.com/optozorax/quad-url/master/js/quad-url.js",
    ),
    (
        "../js/audio.js",
        "https://raw.githubusercontent.com/not-fl3/quad-snd/master/js/audio.js",
    ),
];

fn main() {
    download_all(FONTS);
    download_all(MINIQUAD_JS_FILES);
    embed_levels();
}

/// Download each `(path, url)` whose file doesn't exist yet.
fn download_all(files: &[(&str, &str)]) {
    for &(path, url) in files {
        if Path::new(path).exists() {
            continue;
        }

        eprintln!("Downloading {path}...");

        let response = ureq::get(url)
            .call()
            .unwrap_or_else(|e| panic!("Failed to download {url}: {e}"));

        let mut data = Vec::new();
        response
            .into_body()
            .into_reader()
            .read_to_end(&mut data)
            .unwrap_or_else(|e| panic!("Failed to read {url}: {e}"));

        fs::create_dir_all(Path::new(path).parent().unwrap()).unwrap();
        fs::write(path, &data).unwrap_or_else(|e| panic!("Failed to write {path}: {e}"));

        eprintln!("Downloaded {path}");
    }
}

fn collect_levels(dir: &Path, prefix: &str, levels: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let subdir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let new_prefix = if prefix.is_empty() {
                subdir_name.to_string()
            } else {
                format!("{}/{}", prefix, subdir_name)
            };
            println!("cargo:rerun-if-changed={}", path.display());
            collect_levels(&path, &new_prefix, levels);
        } else if path.extension().is_some_and(|e| e == "csv")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            let level_name = if prefix.is_empty() {
                stem.to_string()
            } else {
                format!("{}/{}", prefix, stem)
            };
            levels.push(level_name);
            println!("cargo:rerun-if-changed={}", path.display());
            let json_path = path.with_extension("json");
            if json_path.exists() {
                println!("cargo:rerun-if-changed={}", json_path.display());
            }
        }
    }
}

fn embed_levels() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("levels.rs");

    let levels_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../levels");

    println!("cargo:rerun-if-changed={}", levels_dir.display());

    let mut levels: Vec<String> = Vec::new();
    collect_levels(&levels_dir, "", &mut levels);

    levels.sort();

    let mut code = String::new();
    code.push_str("pub(crate) static LEVEL_DATA: &[(&str, &str, &str)] = &[\n");

    for name in &levels {
        let rel_csv = format!("../levels/{}.csv", name);
        let rel_json = format!("../levels/{}.json", name);

        code.push_str(&format!(
            "    ({:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{rel_csv}\")), include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{rel_json}\"))),\n",
            name
        ));
    }

    code.push_str("];\n");

    // Only write if content changed to avoid unnecessary recompilation
    let should_write = match fs::read_to_string(&dest_path) {
        Ok(existing) => existing != code,
        Err(_) => true,
    };
    if should_write {
        fs::write(&dest_path, code).unwrap();
    }
}
