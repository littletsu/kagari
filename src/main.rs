
use std::{env, path::Path};
use itertools::{Itertools};
use rfd::FileDialog;
use kagari::config::*;
use path_slash::PathExt as _;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const VERSION: &str = include_str!("version");

fn get_epoch() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs()
}

fn make_fcpxml_from_ranges(ranges: Vec<Vec<i32>>, asset_duration: i32, audio_rate: u32, video_path: String, save_path: &str, fps: i32, w: i64, h: i64) -> String {
    let path = Path::new(&video_path);
    let filename = path.file_name().unwrap().to_str().unwrap();
    let filestem = path.file_stem().unwrap().to_str().unwrap();
    let path_slash = path.to_slash().unwrap();
    let src = path_slash.to_string().replace(" ", "%20");
    let header = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><fcpxml version=\"1.8\"><resources>";
    let format = format!("<format id=\"r1\" frameDuration=\"1/{fps}s\" width=\"{w}\" height=\"{h}\" colorSpace=\"0-0-2\" />");
    let asset_start = audio_frame_to_fps(ranges[0][0], audio_rate, fps);
    let asset = format!("<asset id=\"r2\" name=\"{filename}\" src=\"file://localhost/{src}\" start=\"{asset_start}/{fps}s\" duration=\"{asset_duration}/{audio_rate}s\" hasVideo=\"1\" format=\"r1\" hasAudio=\"1\" audioSources=\"1\" audioChannels=\"2\" audioRate=\"{audio_rate}\" /></resources><library location=\"file://localhost/");
    let mut clips = format!("");
    let mut last_duration = 0;
    for range in ranges {
        let start = audio_frame_to_fps(range[0], audio_rate, fps);
        let end = audio_frame_to_fps(range[1], audio_rate, fps);
        // println!("{} - {}", start, end);
        if range[0] == range[1] {
            continue
        }
        let range_duration = end - start;
        clips = format!("{clips}<asset-clip name=\"{filename}\" offset=\"{last_duration}/{fps}s\" ref=\"r2\" start=\"{start}/{fps}s\" duration=\"{range_duration}/{fps}s\" audioRole=\"dialogue\" format=\"r1\" tcFormat=\"NDF\" />");
        last_duration += range_duration;
    }
    let event_project_sequence = format!("\"><event name=\"{filestem}\"><project name=\"{filestem}\"><sequence duration=\"{last_duration}/{fps}s\" format=\"r1\" tcStart=\"0/{fps}s\" tcFormat=\"NDF\" audioLayout=\"stereo\" audioRate=\"{audio_rate}\"><spine>");

    let end = "</spine></sequence></project></event></library></fcpxml>";
    let library = Path::new(save_path).to_slash().unwrap().to_string();
    format!("{header}{format}{asset}{library}{event_project_sequence}{clips}{end}")
}

fn version() -> f32 {
    VERSION.parse::<f32>().unwrap()
}

fn latest_version() -> f32 {
    println!("Comprobando actualizaciones...");
    let request = reqwest::blocking::get("https://raw.githubusercontent.com/littletsu/kagari/master/src/version");
    let version = match request {
        Ok(response) => {
            let parse = response.text().unwrap().parse::<f32>();
            match parse {
                Ok(latest) => {
                    latest
                }, 
                Err(_) => {
                    println!("Hubo un error al procesar la version del servidor.");
                    version()
                }
            }
        },
        Err(_) => {
            println!("Hubo un error al obtener la version del servidor.");
            version()
        }
    };
    version
}

fn get_changelog() -> String {
    let request = reqwest::blocking::get("https://raw.githubusercontent.com/littletsu/kagari/master/changelog");
    let changelog = match request {
        Ok(response) => {
            response.text().unwrap()
        },
        Err(_) => {
            String::from("Hubo un error al obtener la lista de cambios.")
        }
    };
    changelog
}

fn download_version(version: f32) -> bool {
    let request = reqwest::blocking::get(format!("https://github.com/littletsu/kagari/releases/download/{version}/kagari.exe"));
    let result = match request {
        Ok(response) => {
            let text = response.bytes().unwrap();

            let bin_name = env::args().nth(0).unwrap();
            let bin_filename = Path::new(&bin_name).file_name().unwrap().to_str().unwrap();
            let bin_new_path = bin_name.replace(bin_filename, "kagari_old");
            std::fs::rename(&bin_name, &bin_new_path).unwrap();

            let mut file = std::fs::File::create(bin_name).unwrap();
            let mut content =  std::io::Cursor::new(text);
            std::io::copy(&mut content, &mut file).unwrap();
            
            true
        },
        Err(_) => {
            println!("Hubo un error al descargar la nueva version.");
            false
        }
    };
    result
}
fn main() {
    println!("Kagari Version {VERSION}");
    let latest = latest_version();
    if latest > version() {
        println!("Cambios de la version {}:\n{}", latest, get_changelog());
        println!("Hay una nueva version de Kagari ({}). Actualizando", latest);
        if download_version(latest) {
            println!("Se actualizo correctamente. Puedes cerrar esta ventana.");
            loop {

            }
        } else {
            println!("No se pudo descargar la actualizacion correctamente.");
        }

    }
    let default_config = Config {
        detection: DetectionConfig {
            energy: 24000.0,
            sample_chunks_ms: 500
        }
    };
    let config = Config::from_file("kagari.toml", default_config);
    
    let file = env::args().nth(1).unwrap();
    let filepath = Path::new(&file);
    let filename = filepath.file_name().unwrap().to_str().unwrap();
    let wav_out = format!("{}.wav", get_epoch());
    let wav_path = filepath.to_str().unwrap().replace(filename, &wav_out);
    println!("Decodificando audio...");
    Command::new("ffmpeg")
            .args(["-i", &file, "-q:a", "0", "-map", "a", "-ar", "48000", &wav_out])
            .output()
            .expect("failed to execute process");
    println!("Leyendo propiedades del video...");
    let video = &ffprobe::ffprobe(&file).unwrap().streams[0];
    let mut video_fr = video.r_frame_rate.split("/");
    let fr_a = video_fr.next().unwrap().parse::<f32>().unwrap();
    let fr_b = video_fr.next().unwrap().parse::<f32>().unwrap();
    let fps = (fr_a / fr_b).ceil() as i32;
    // println!("{}", fps);
    let mut reader = hound::WavReader::open(&wav_path).unwrap();
    let spec = reader.spec();
    println!("{}", spec.bits_per_sample);
    let samples = reader.samples::<i16>();
    let sample_chunks = config.detection.sample_chunks_ms as f32 * (spec.sample_rate as f32 / 1000.0);
    let chunks = &samples.chunks(sample_chunks as usize);

    let mut i = 0;
    let mut energy_samples: Vec<i16> = Vec::new();
    let mut silent_ranges: Vec<Vec<i32>> = Vec::new();
    let mut range: Vec<i32> = Vec::new();
    println!("Analizando muestras de audio...");
    for chunk in chunks {
        let mut energy: f32 = 0.0;
        let mut chunk_samples: Vec<i16> = Vec::new();
        for result in chunk {
            let sample = result.unwrap();
            chunk_samples.push(sample);
            i += 1;
            energy += sample as f32 * sample as f32
        }
        energy = f32::sqrt(energy);
        // println!("{}", energy);
        if energy > config.detection.energy {
            range.push(i);
            energy_samples.append(&mut chunk_samples)
        } else {
            if range.len() != 0 {
                let start = range[0] - ((chunk_samples.len() as i32));
                let end = range[range.len() - 1] + ((chunk_samples.len() as i32) / 2);
                silent_ranges.push(vec![start, end]);
                range.clear();
            }
            
        }
        
    }

    let mut writer = hound::WavWriter::create("out.wav", spec).unwrap();

    for sample in energy_samples {
        writer.write_sample(sample).unwrap();
    }

    // println!("{} {} {} {}", spec.sample_rate, sample_chunks, sample_chunks as usize, config.detection.sample_chunks_ms);
    println!("Elige donde guardar el archivo fcpxml");
    let save_filename = format!("{filename}.fcpxml");
    let save = FileDialog::new()
        .set_file_name(&save_filename)
        .save_file()
        .unwrap();
    println!("Guardando...");
    let save_path = save.to_str().unwrap();
    let xml = make_fcpxml_from_ranges(silent_ranges, i, spec.sample_rate, file, save_path, fps, video.width.unwrap(), video.height.unwrap());
    // println!("{}", xml);
    std::fs::write(save_path, xml).unwrap();
    println!("Borrando archivos temporales...");
    std::fs::remove_file(wav_path).unwrap();
    println!("Finalizado. Puedes cerrar esta ventana.");
    loop {
        
    }
    
}

fn audio_frame_to_fps(chunk: i32, audio_rate: u32, video_fps: i32) -> i32 {
    ((chunk as f32 / 2.0 / audio_rate as f32) * video_fps as f32) as i32
}