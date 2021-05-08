use notify::{*, DebouncedEvent::*};
use std::sync::mpsc::channel;
use std::{fs, env, path::*, time::Duration, result::Result};
use rm_rf::*;
use fs_extra::*;

fn get_actual_path(parent: &Path, file: &PathBuf, orig: &Path) -> Result<PathBuf, StripPrefixError> {
    //println!("Calling get_actual_path on {}, {}, {}", parent.display(), file.display(), orig.display());
    let sp = file.strip_prefix(orig);
    match sp {
        Ok(p) => { 
            return Result::Ok(parent.join(p.to_str().unwrap_or_default()));
        },
        Err(e) => {
            println!("Strip Prefix Error: {:?}", e);
            return Result::Err(e);
        }
    }
}

fn main() {
    let from_path_str = env::args().nth(1).expect("No From Path given");
    let to_path_str = env::args().nth(2).expect("No To Path given");
    let from_path = Path::new(from_path_str.as_str()).canonicalize().unwrap();
    let to_path = Path::new(to_path_str.as_str()).canonicalize().unwrap();
    match from_path.exists() {
        true => (),
        false => panic!("from_path does not exist!")
    }
    match to_path.exists() {
        true => (),
        false => panic!("to_path does not exist!")
    }
    let (tx, rx) = channel();
    let watch_path = from_path.canonicalize().unwrap();
    let watch_path_str = watch_path.as_os_str().to_str().unwrap();
    let options = dir::CopyOptions {
        overwrite: true,
        skip_exist: false,
        buffer_size: 64000,
        copy_inside: true,
        depth: 0,
        content_only: false
    };

    let mut watcher = watcher(tx, Duration::from_secs(3)).unwrap();
    watcher.watch(watch_path_str, RecursiveMode::Recursive).unwrap();
    println!("Listening on {}", watch_path_str);
    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    Create(p) | Write(p) => {
                        let mut actual_to_path = get_actual_path(&to_path, &p, &from_path).unwrap();
                        match actual_to_path.is_dir() {
                            true => (),
                            false => {
                                actual_to_path = actual_to_path.parent().unwrap_or(&actual_to_path).to_path_buf();
                            }
                        }
                        //println!("actual path: {}", &actual_to_path.display());
                        match copy_items(&[&p], &actual_to_path, &options) {
                            Ok(t) => println!("Copied {} bytes from {} to {}", t, &p.display(), &actual_to_path.display()),
                            Err(e) => println!("Error copying: {:?}", e)
                        }
                    },
                    Remove(p) => {
                        let actual_to_path = get_actual_path(&to_path, &p, &from_path).unwrap();
                        match remove(&actual_to_path) {
                            Ok(_) => println!("Removed file or directory {}", &actual_to_path.display()),
                            Err(e) => println!("Error deleting: {:?}", e)
                        }
                    },
                    Rename(p, q) => {
                        let source_path = get_actual_path(&to_path, &p, &from_path).unwrap();
                        let dest_path = get_actual_path(&to_path, &q, &from_path).unwrap();
                        match fs::rename(source_path, dest_path) {
                            Ok(_) => {},
                            Err(e) => println!("Error renaming: {:?}", e)
                        }
                    },
                    Chmod(p) => {
                        let orig_metadata = fs::metadata(&p);
                        match orig_metadata {
                            Ok(md) => { 
                                fs::set_permissions(get_actual_path(&to_path, &p, &from_path).unwrap(), md.permissions()).ok();
                            }
                            Err(e) => {
                                println!("Error chmodding: {:?}", e);
                            }
                        }
                    },
                    Error(e, p) => {
                        println!("An error occurred watching path '{}': {:?}", p.unwrap_or_default().as_path().display(), e);
                    },
                    _ => ()
                }
            },
            Err(e) => println!("Watch error: {:?}", e)
        }
    }
}
