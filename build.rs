extern crate curl;
extern crate glob;
extern crate gcc;
extern crate pkg_config;
extern crate cmake;
extern crate toml;

use std::process::Command;
use std::path::{ Path, PathBuf };
use std::env;
use std::fs;
use std::fs::{ File, read_dir };
use std::fs::OpenOptions;
use std::ffi::OsString;
use std::io::{Read, Write};

use curl::easy;
use glob::glob;

const PKG_CONFIG_PATH_ENV_VAR: &'static str = "PKG_CONFIG_PATH";
const CARGO_FILE_PATH: &'static str = "Cargo.toml";

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    if build_required(&out_dir) {
        println!("Building opencv.");
        clean_previous_build(&out_dir)
            .and_then( |_| opencv_src_url() )
            .and_then( |opencv_source_url| download_opencv_source(&out_dir, &opencv_source_url) )
            .and_then( |opencv_archive_path| extract_opencv_source(&out_dir, &opencv_archive_path) )
            .and_then( |source_dir| build_opencv(&out_dir, &source_dir) )
            .and_then( |_| fix_lib_png(&out_dir) )
            .expect("Failed to build opencv via build script.");
    }

    if feature_generation_required() {
        println!("Generating opencv-rust features.");
        opencv_src_url()
            .and_then( |opencv_source_url| download_opencv_source(&out_dir, &opencv_source_url) )
            .and_then( |opencv_archive_path| extract_opencv_source(&out_dir, &opencv_archive_path) )
            .and_then( |source_dir| write_features_to_cargo_toml(&source_dir) )
            .expect("Failed to generate opencv-rust features via build script.");
    }

    let (opencv_pkg_info, opencv_path) = find_opencv_libs(&out_dir)
        .expect("Could not find the opencv libs.");

    println!("OpenCV lives in {:?}", opencv_path);
    println!("Generating code in {:?}", out_dir);

    let mut gcc = gcc::Config::new();
    gcc.flag("-std=c++0x");
    for path in opencv_pkg_info.include_paths {
        gcc.include(path);
    }

    let modules = vec![
    ("core", vec!["core/types_c.h", "core/core.hpp" ]), // utility, base
    ("imgproc", vec![
    "imgproc/types_c.h",
    "imgproc/imgproc_c.h",
    "imgproc/imgproc.hpp"
    ]),
    ("highgui", vec![
    "highgui/cap_ios.h",
    "highgui/highgui.hpp",
    "highgui/highgui_c.h",
    //"highgui/ios.h"
    ]),
    ("features2d", vec![ "features2d/features2d.hpp" ]),
    ("photo", vec!["photo/photo_c.h", "photo/photo.hpp" ]),
    ("video", vec![
    "video/tracking.hpp",
    "video/video.hpp",
    "video/background_segm.hpp"
    ]),
    ("objdetect", vec![ "objdetect/objdetect.hpp" ]),
    ("calib3d", vec![ "calib3d/calib3d.hpp"])
    ];

    let mut types = PathBuf::from(&out_dir);
    types.push("common_opencv.h");
    {
        let mut types = File::create(types).unwrap();
        for ref m in modules.iter() {
            write!(&mut types, "#include <opencv2/{}/{}.hpp>\n", m.0, m.0).unwrap();
        }
    }

    let mut types = PathBuf::from(&out_dir);
    types.push("types.h");
    {
        let mut types = File::create(types).unwrap();
        write!(&mut types, "#include <cstddef>\n").unwrap();
    }

    for ref module in modules.iter() {
        let mut cpp = PathBuf::from(&out_dir);
        cpp.push(module.0);
        cpp.set_extension("cpp");

        if !Command::new("python2.7")
            .args(&["gen_rust.py", "hdr_parser.py", &*out_dir, module.0])
            .args(&(module.1.iter().map(|p| {
                let mut path = opencv_path.clone();
                path.push(p);
                path.into_os_string()
            }).collect::<Vec<OsString>>()[..]))
            .status().unwrap().success() {
            panic!();
        }

        gcc.file(cpp);
    }

    let mut return_types = PathBuf::from(&out_dir);
    return_types.push("return_types.h");
    let mut hub_return_types = File::create(return_types).unwrap();
    for entry in glob(&(out_dir.clone() + "/cv_return_value_*.type.h")).unwrap() {
        writeln!(&mut hub_return_types, r#"#include "{}""#,
                 entry.unwrap().file_name().unwrap().to_str().unwrap()).unwrap();
    }

    for entry in glob("native/*.cpp").unwrap() {
        gcc.file(entry.unwrap());
    }
    for entry in glob(&(out_dir.clone() + "/*.type.cpp")).unwrap() {
        gcc.file(entry.unwrap());
    }

    gcc.cpp(true).include(".").include(&out_dir)
        .flag("-Wno-c++11-extensions");

    gcc.compile("libocvrs.a");

    for ref module in &modules {
        println!("Compiling = {:?}", module.0);
        let e = Command::new("sh").current_dir(&out_dir).arg("-c").arg(
            format!("g++ {}.consts.cpp -o {}.consts `pkg-config --cflags --libs opencv`",
                    module.0, module.0)
        ).status().unwrap();
        assert!(e.success());
        let e = Command::new("sh").current_dir(&out_dir).arg("-c").arg(
            format!("./{}.consts > {}.consts.rs", module.0, module.0)
        ).status().unwrap();
        assert!(e.success());
    }

    let mut hub_filename = PathBuf::from(&out_dir);
    hub_filename.push("hub.rs");
    {
        let mut hub = File::create(hub_filename).unwrap();
        for ref module in &modules {
            writeln!(&mut hub, r#"pub mod {};"#, module.0).unwrap();
        }
        writeln!(&mut hub, r#"pub mod types {{"#).unwrap();
        writeln!(&mut hub, "  use libc::{{ c_void, c_char, size_t }};").unwrap();
        for entry in glob(&(out_dir.clone() + "/*.type.rs")).unwrap() {
            writeln!(&mut hub, r#"  include!(concat!(env!("OUT_DIR"), "/{}"));"#,
                     entry.unwrap().file_name().unwrap().to_str().unwrap()).unwrap();
        }
        writeln!(&mut hub, r#"}}"#).unwrap();
        writeln!(&mut hub, "#[doc(hidden)] pub mod sys {{").unwrap();
        writeln!(&mut hub, "  use libc::{{ c_void, c_char, size_t }};").unwrap();
        for entry in glob(&(out_dir.clone() + "/*.rv.rs")).unwrap() {
            writeln!(&mut hub, r#"  include!(concat!(env!("OUT_DIR"), "/{}"));"#,
                     entry.unwrap().file_name().unwrap().to_str().unwrap()).unwrap();
        }
        for ref module in &modules {
            writeln!(&mut hub, r#"  include!(concat!(env!("OUT_DIR"), "/{}.externs.rs"));"#, module.0).unwrap();
        }
        writeln!(&mut hub, "}}\n").unwrap();
    }
    println!("cargo:rustc-link-lib=ocvrs");
}

type BuildResult<T> = Result<T, String>;
const EXTRACTOR: &'static str = "unzip";

fn opencv_src_url() -> BuildResult<String> {
    parse_cargo_toml()?
        .get("package")
        .and_then( |pkg| pkg.as_table() )
        .and_then( |pkg_tbl| pkg_tbl.get("metadata") )
        .and_then( |meta| meta.as_table() )
        .and_then( |meta_tbl| meta_tbl.get("opencv-src") )
        .and_then( |src| src.as_str() )
        .map( |src_url| src_url.to_owned() )
        .ok_or(format!("Did not find the opencv source url in {}.", CARGO_FILE_PATH))
}

fn clean_previous_build(out_dir: &str) -> BuildResult<()> {
    println!("Cleaning {}.", out_dir);
    fs::remove_dir_all(out_dir)
        .map_err(|_| format!("Failed to remove {}.", out_dir))?;
    fs::create_dir(out_dir)
        .map_err(|_| format!("Failed to create {}.", out_dir))
}

fn download_opencv_source(out_dir: &str, opencv_source_url: &str) -> BuildResult<String> {
    opencv_source_url.split("/")
        .last()
        .ok_or(format!("Unabled to extract the opencv source archive name from {}.", &opencv_source_url))
        .and_then(|archive_name| {
            let opencv_archive_path = format!("{}/opencv-{}", out_dir, archive_name);
            if fs::metadata(&opencv_archive_path).is_err() {
                println!("Downloading {}.", &opencv_source_url);
                let mut curl = easy::Easy::new();
                curl.url(opencv_source_url)
                    .or(Err(format!("Failed to set the OpenCV source URL {}.", opencv_source_url)))?;
                curl.follow_location(true)
                    .or(Err(format!("Failed to set follow location on download of {}.", opencv_source_url)))?;
                let mut file = fs::OpenOptions::new()
                    .read(true)
                    .append(true)
                    .create(true)
                    .open(&opencv_archive_path)
                    .or(Err(format!("Unable to open the destination file {}.", &opencv_archive_path)))?;
                let mut transfer = curl.transfer();
                transfer.write_function(|data| {
                    file.write(&data).map_err(|_| easy::WriteError::Pause)
                }).or(Err(format!("Unable to write to file {}.", &opencv_archive_path)))?;
                transfer.perform()
                    .or(Err(format!("Failed to download {}.", opencv_source_url)))?;
                println!("Downloaded {} to {}.", &opencv_source_url, &opencv_archive_path);
            }

            Ok(opencv_archive_path)
        })
}

fn extract_opencv_source(out_dir: &str, archive_path: &str) -> BuildResult<String> {
    let src_path = Path::new(archive_path)
        .file_stem()
        .and_then( |dir_name| dir_name.to_str() )
        .map( |dir_name_str| format!("{}/{}", out_dir, dir_name_str) )
        .ok_or(format!("Failed to compute the extracted directory name for {}.", archive_path))?;

    if fs::metadata(&src_path).is_err() {
        println!("Extracting {}.", archive_path);
        Command::new(EXTRACTOR)
            .current_dir(out_dir)
            .arg(archive_path)
            .status()
            .map_err(|_| "Failed to run the extractor.".to_string())
            .and_then(|exit_status| {
                if exit_status.success() {
                    Ok(())
                } else {
                    Err(format!("Failed to extract {}.", archive_path))
                }
            })?;
    }

    Ok(src_path)
}

fn build_opencv(out_dir: &str, src_dir: &str) -> BuildResult<()> {
    parse_features(src_dir, |feats| {
        let dist_dir = dist_dir(out_dir);
        let mut cmake = cmake::Config::new(src_dir);
        cmake.define("CMAKE_BUILD_TYPE", "Release");
        cmake.define("CMAKE_INSTALL_PREFIX", &dist_dir);
        for feat in feats {
            let feat_on = env::var(format!("CARGO_FEATURE_{}_ON", feat.name)).is_ok();
            let feat_off = env::var(format!("CARGO_FEATURE_{}_OFF", feat.name)).is_ok();
            match (feat_on, feat_off) {
                (true, true) => Err(
                    format!("Feature {} was set to both ON and OFF. Check your feature flags.",
                            feat.name
                    )
                ),
                (true, false) => {
                    cmake.define(format!("{}:BOOL", feat.name), "ON");
                    Ok(())
                },
                (false, true) => {
                    cmake.define(format!("{}:BOOL", feat.name), "OFF");
                    Ok(())
                },
                (false, false) => Ok(())
            }?;
        }

        cmake.build();

        Ok(())
    })
}

// For some reason the static build of libpng is written to disk with the name liblibpng.a which
// causes linking to fail because opencv.pc specifies libpng as -lpng. So we make a copy so both
// names exist.
fn fix_lib_png(out_dir: &str) -> BuildResult<()> {
    let dist_dir = dist_dir(out_dir);
    let src = format!("{}/share/OpenCV/3rdparty/lib/liblibpng.a", &dist_dir);
    let dst = format!("{}/share/OpenCV/3rdparty/lib/libpng.a", &dist_dir);
    fs::copy(&src, &dst)
        .map_err( |_| format!("Failed to copy {} to {}.", src, dst) )
        .map(|_| ())
}

fn find_opencv_libs(out_dir: &str) -> BuildResult<(pkg_config::Library, PathBuf)> {
    let pkg_config_path = format!("{}:{}/dist/lib/pkgconfig",
                                  env::var(PKG_CONFIG_PATH_ENV_VAR).unwrap_or("/usr/local/lib/pkgconfig/".to_string()),
                                  &out_dir);
    env::set_var(PKG_CONFIG_PATH_ENV_VAR, &pkg_config_path);
    pkg_config::Config::new()
        .statik(true)
        .probe("opencv")
        .map_err(|_| format!("Could not find the opencv pkg information in {}.", &pkg_config_path))
        .and_then( |opencv_lib_info| {
            let mut search_paths = opencv_lib_info.include_paths.clone();
            search_paths.push(PathBuf::from("/usr/include"));
            search_paths.iter()
                .map( |p| {
                    let mut path = PathBuf::from(p);
                    path.push("opencv2");
                    path
                })
                .find( |path| {
                    println!("Checking include path {} for opencv.", path.to_string_lossy());
                    read_dir(path).is_ok()
                })
                .map_or(Err("Could not find opencv on any of the includes paths.".to_string()), |opencv_path| Ok((opencv_lib_info, opencv_path)))
        })
}

fn build_required(out_dir: &str) -> bool {
    env::var("CARGO_FEATURE_BUILD").is_ok() &&
        fs::metadata(format!("{}/dist/lib/pkgconfig/opencv.pc", out_dir)).is_err() ||
        env::var("FORCE_OPENCV_BUILD").is_ok()
}

fn dist_dir(out_dir: &str) -> String {
    format!("{}/dist", out_dir)
}

fn feature_generation_required() -> bool {
    env::var("CARGO_FEATURE_GEN_FEATS").is_ok()
}

const OPTION_TOKEN: &'static str = "OCV_OPTION(";
fn write_features_to_cargo_toml(src_dir: &str) -> BuildResult<()> {
    parse_features(src_dir, |feats| {
        let mut feats_buf = String::new();
        feats_buf.push_str("[features]\n");
        feats_buf.push_str("build = []\n");
        for feat in feats {
            feats_buf.push_str(&format!("{}\n", &feat.to_cargo_feature()))
        }

        let cargo_file_content = cargo_file_content()?
            .split("[features]")
            .next()
            .map(|const_content| const_content.trim().to_string())
            .ok_or("Could not split the Cargo content at the features section.")?;

        let new_cargo_file_content = format!("{}\n{}", &cargo_file_content, &feats_buf);
        println!("{}", &new_cargo_file_content);

        OpenOptions::new()
            .write(true)
            .open(CARGO_FILE_PATH)
            .map_err(|e| format!("Failed to open {} for writing: {}.", CARGO_FILE_PATH, e))
            .and_then(|mut file| {
                file.write(new_cargo_file_content.as_bytes())
                    .map_err(|e| format!("Failed to write {}: {}.", CARGO_FILE_PATH, e))
                    .map(|_| ())
            })
    })
}

fn parse_features<F>(src_dir: &str, feat_handler: F) -> BuildResult<()>
    where F: Fn(Vec<OpenCVFeature>) -> BuildResult<()> {

    let opencv_cmake_file_path = format!("{}/CMakeLists.txt", src_dir);
    let cmake_content = cmake_content(&Path::new(&opencv_cmake_file_path))?;

    feat_handler(
        cmake_content.lines()
            .filter( |line| line.starts_with(OPTION_TOKEN) )
            .map( |feature_line| parse_feature(&feature_line) )
            .collect::<Vec<_>>()
    )
}

fn cmake_content(cmake_path: &Path) -> BuildResult<String> {
    fs::File::open(cmake_path)
        .map_err( |e| format!("Failed to open {} with error: {}.", cmake_path.to_string_lossy(), e) )
        .and_then(|mut file| {
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| format!("Failed to read {} with error: {}.", cmake_path.to_string_lossy(), e) )
                .map(|_| contents)
        })
}

#[derive(Debug)]
struct OpenCVFeature<'a> {
    name: &'a str,
    default: &'a str,
}

impl<'a> OpenCVFeature<'a> {

    fn to_cargo_feature(&self) -> String {
        let mut feat = String::new();
        feat.push_str(
            &format!("{}_on = [\"build\"] #default = {}\n",
                     &self.name.to_lowercase().trim(),
                     &self.default
            )
        );
        feat.push_str(
            &format!("{}_off = [\"build\"] #default = {}\n",
                     &self.name.to_lowercase().trim(),
                     &self.default
            )
        );
        feat
    }
}

fn parse_feature(feature: &str) -> OpenCVFeature {
    let parsed_feature = feature
        .trim_left_matches(OPTION_TOKEN)
        .splitn(3, "\"")
        .collect::<Vec<_>>();

    OpenCVFeature {
        name: parsed_feature[0].trim(),
        default: parsed_feature[2].trim_right_matches(")").trim(),
    }
}

fn parse_cargo_toml() -> BuildResult<toml::Table> {
    let raw_toml = cargo_file_content()?;
    toml::Parser::new(&raw_toml)
        .parse()
        .ok_or(format!("Failed to parse opencv source url from {}.", CARGO_FILE_PATH))
}

fn cargo_file_content() -> BuildResult<String> {
    let cargo_file_path = Path::new(CARGO_FILE_PATH);
    fs::File::open(&cargo_file_path)
        .map_err( |e| format!("Failed to open {} with error: {}.", CARGO_FILE_PATH, e) )
        .and_then(|mut file| {
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| format!("Failed to read config file {} with error: {}.", CARGO_FILE_PATH, e) )
                .map(|_| contents)
        })
}