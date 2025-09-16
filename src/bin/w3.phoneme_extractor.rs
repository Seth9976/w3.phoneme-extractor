extern crate logger;
#[macro_use]
extern crate log;

extern crate getopts;

extern crate w3phonemetools;

use getopts::{Matches, Options};
use std::env;
use std::path::{Path, PathBuf};

use logger::LevelFilter;

use w3phonemetools::{ActorMapping ,ProcessingQueue};

use w3phonemetools::gui;
// ----------------------------------------------------------------------------
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const NAME: &str = "w3speech phonemes extractor";
const HELPFILE: &str = "./help-phoneme-extractor.txt";

const LIBRARIES: &str =
    "This program uses the CMU Pocketsphinx library (https://github.com/cmusphinx/pocketsphinx),\
     \nthe eSpeak Library (http://espeak.sourceforge.net).";

#[derive(PartialEq)]
enum OpMode {
    Interactive,
    Extract,
    LogMissing,
    Generate,
}

struct CliArgs {
    mode: OpMode,
    language: String,
    force_rename: bool,
    input: Option<PathBuf>,
    strings_file: Option<PathBuf>,
    mappings_file: Option<PathBuf>,
    worker: Option<usize>,
    datadir: PathBuf,
    outdir: Option<PathBuf>,
    loglevel: LevelFilter,
}
// ----------------------------------------------------------------------------
fn setup_option() -> Options {
    let mut opts = Options::new();

    // misc
    opts.optflag("h", "help", "print this help menu");

    // main option
    opts.optopt(
        "e",
        "extract",
        "non interactive (batch-mode without gui) extraction \
         of timed phoneme information from <id>*.ogg and <id>*.wav file(s) found \
         in DIRECTORY. results are saved as <id>.phonemes file(s) in DIRECTORY.\
         requires a strings-file with matching text lines for every <id>. \
         if no --strings-file parameter is given the first found csv-file \
         (*.csv|*.strings-csv) in DIRECTORY or its parent directory is used.",
        "DIRECTORY",
    );

    // input stringsfile (required for batch mode)
    opts.optopt(
        "s",
        "strings-file",
        "csv file with <id>s and their associated text lines. format as \
         described in the GUI help.",
        "FILE.csv",
    );

    // optional mode
    opts.optflag(
        "",
        "generate-from-text-only",
        "generate phonemes file for every id found in strings-file based solely \
         on the text.",
    );

    opts.optopt(
        "a",
        "audio-dir",
        "defines the starting directory containing audio files (see --extract) \
         to be opened in gui mode. default is current directory. required \
         strings-file will be searched as in batch-mode or can be provided \
         with --strings-file.",
        "DIRECTORY",
    );

    // data directory
    opts.optopt(
        "d",
        "data-dir",
        "defines data-directory containing pocketsphinx and eSpeak data. \
         default is \"./data\".",
        "DIRECTORY",
    );

    // output directory
    opts.optopt(
        "o",
        "output-dir",
        "defines the output directory for generated phonemes. valid only in \
         combination with --generate-from-text-only. default is directory \
         of the strings csv files.",
        "DIRECTORY",
    );

    // number of worker threads
    opts.optopt(
        "w",
        "worker-threads",
        "defines number of phoneme extracting worker-threads in interactive gui \
         mode. max is 16, default is 1",
        "COUNT",
    );

    // autorenaming of files
    opts.optflag(
        "",
        "force-rename",
        "will rename all audio files that have an <id> prefix automatically to \
         <id>[<duration>]<actor><texthint>.<extension>. duration is extractred from the \
         audiofile. texthint is the (shortened) textline from the strings-csv. \
         actor will only be added if it can be extracted from the strings-csv.",
    );

    // language
    opts.optopt(
        "l",
        "language",
        "language code which defines the used speech/phoneme \
         recognition models and text to phoneme translation. default is \"en\". NOTE: \
         the code must be supported by eSpeak and CMUSphinx. It will be lowercased and \
         mapped to a data directory \"data/pocketsphinx/<LANGUAGE>\" which must exist and \
         contain the appropriate pocketsphinx models, see readme.txt in data/pocketsphinx \
         directory.",
        "LANGUAGE",
    );

    // actor mappings
    opts.optopt(
        "",
        "actor-mappings",
        "optional mappings of actors to alias names. format is a case \
         insensitive, colon separated two column mapping: <actor>:<alias> \
         Note: file will be updated/overriden if any undefined actor ids are found \
         during processing of files or if mappings are changed (e.g. in the GUI).",
        "FILE",
    );

    // autorenaming of files
    opts.optflag(
        "",
        "log-missing-audio",
        "matches found audio files with <id> prefix to ids from strings file \
         and logs all ids from strings file without audiofile into a new csv in \
         the strings csv file location.",
    );

    // misc
    opts.optflag("v", "verbose", "show debug messages in console");
    opts.optflag("", "very-verbose", "show more debug messages in console");

    opts
}
// ----------------------------------------------------------------------------
fn check_dir<T: Into<PathBuf>>(dir: T, name: &str) -> Result<PathBuf, String> {
    let dir = dir.into();
    if !dir.exists() || !dir.is_dir() {
        Err(format!("{} [{}] does not exist", name, dir.display()))
    } else {
        Ok(dir)
    }
}
// ----------------------------------------------------------------------------
fn check_file<T: Into<PathBuf>>(file: T, errname: &str) -> Result<PathBuf, String> {
    let file = file.into();
    if !file.exists() || !file.is_file() {
        Err(format!("{} [{}] does not exist", errname, file.display()))
    } else {
        Ok(file)
    }
}
// ----------------------------------------------------------------------------
fn parse_arguments(found: Matches) -> Result<CliArgs, String> {
    let loglevel = if found.opt_present("very-verbose") {
        LevelFilter::Trace
    } else if found.opt_present("v") {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    let _ = logger::init(loglevel);

    // extract options (at most one of these)
    let param_extract_dir = found.opt_str("e");
    let param_generated_only = found.opt_present("generate-from-text-only");
    let param_force_rename = found.opt_present("force-rename");
    let param_log_missing = found.opt_present("log-missing-audio");
    let param_audio_dir = found.opt_str("a");
    let param_worker = found.opt_str("w");
    let param_mappings = found.opt_str("actor-mappings");

    // dirs
    let param_data_dir = found.opt_str("d");
    let param_out_dir = found.opt_str("o");

    // strings file
    let param_strings_file = found.opt_str("s");

    // -- check for invalid settings
    let mut modes = 0;
    if param_generated_only {
        modes += 1;
    }
    if param_extract_dir.is_some() {
        modes += 1;
    }
    if param_log_missing {
        modes += 1;
    }

    if modes > 1 {
        return Err("invalid combination of options: choose either \
                    --extract, --generate-from-text-only or --log-missing-audio"
            .to_string());
    }
    if param_audio_dir.is_some() && (param_generated_only || param_extract_dir.is_some()) {
        return Err("invalid combination of options: audio-dir option is only \
                    valid for interactive gui mode or --log-missing-audio \
                    (not in --extract or --generate-from-text-only batch mode"
            .to_string());
    }
    if param_out_dir.is_some() && !param_generated_only {
        return Err("invalid combination of options: output-dir option is only \
                    valid for --generate-from-text-only batch mode"
            .to_string());
    }
    if param_generated_only && param_strings_file.is_none() {
        return Err("--generate-from-text-only requires a --strings-file parameter".to_string());
    }

    // -- set options or defaults
    let (extract_dir, extract_dir_lang) = match param_extract_dir {
        Some(dir) => {
            let audio_dir = check_dir(dir.as_str(), "audio directory")?;
            debug!("audio directory provided. extracting language prefix...");
            let lang = w3phonemetools::extract_language_info(&audio_dir);
            (Some(audio_dir), lang)
        }
        None => (None, None),
    };

    let (audio_dir, audio_dir_lang) = match param_audio_dir {
        Some(audio_dir) => {
            let audio_dir = check_dir(audio_dir.as_str(), "audio directory")?;
            debug!("audio directory provided. extracting language prefix...");
            let lang = w3phonemetools::extract_language_info(&audio_dir);
            (Some(audio_dir), lang)
        }
        None => (None, None),
    };

    let outdir = match param_out_dir {
        Some(dir) => Some(check_dir(dir.as_str(), "output directory")?),
        None => None,
    };

    let strings_file = match param_strings_file {
        Some(ref file) => Some(check_file(file, "strings file")?),
        None => None,
    };

    let mappings_file = match param_mappings {
        Some(ref file) => Some(check_file(file, "actor mappings file")?),
        None => None,
    };

    let language = match found.opt_str("l") {
        Some(lang_code) => lang_code.to_lowercase(),
        None => {
            match extract_dir_lang.or(audio_dir_lang).take() {
                Some(lang) => {
                    debug!("setting language to detected: {lang}");
                    lang
                }
                None => {
                    debug!("setting language to default: en");
                    "en".to_owned()
                }
            }
        }
    };

    let worker = match param_worker {
        Some(value) => {
            let worker = value
                .parse::<usize>()
                .map_err(|e| format!("could not parse worker-threads parameter: {}", e))?;
            if worker > 16 {
                return Err("max 16 worker-threads allowed".to_string());
            }
            Some(worker.max(1))
        }
        None => None,
    };

    let datadir = check_dir(
        param_data_dir.unwrap_or_else(|| {
            debug!("using default datadir: ./data");
            String::from("./data")
        }),
        "data directory",
    )?;

    // set operation mode
    let (mode, input) = if param_generated_only {
        (OpMode::Generate, None)
    } else if extract_dir.is_some() {
        (OpMode::Extract, extract_dir)
    } else if param_log_missing {
        (OpMode::LogMissing, audio_dir)
    } else {
        (OpMode::Interactive, audio_dir)
    };

    if worker.is_some() && mode != OpMode::Interactive {
        return Err("--worker-threads only valid in interactive gui mode".to_string());
    }

    if param_force_rename && mode != OpMode::Extract {
        return Err("--force-rename only valid in --extract mode".to_string());
    };

    Ok(CliArgs {
        mode,
        language,
        force_rename: param_force_rename,
        input,
        strings_file,
        mappings_file,
        worker,
        datadir,
        outdir,
        loglevel,
    })
}
// ----------------------------------------------------------------------------
fn print_usage(program: &str, opts: Options) {
    let brief = format!("\nUsage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}
// ----------------------------------------------------------------------------
fn extract_phonemes(
    inputdir: PathBuf,
    stringsfile: Option<PathBuf>,
    datadir: PathBuf,
    language: String,
    actor_mappings_file: Option<&Path>,
    force_rename: bool,
    loglevel: LevelFilter,
) -> Result<(), String> {
    info!(
        "EXTRACTING PHONEMES: SCANNING {} for audio",
        inputdir.display()
    );
    if force_rename {
        info!("> audiofiles will be automatically renamed.");
    }

    let stringsfile =
        stringsfile.map_or_else(|| w3phonemetools::search_strings_file(&inputdir), Ok)?;

    let mut processor =
        w3phonemetools::init_phoneme_extraction(&language, &stringsfile, &datadir, loglevel)?;

    // init mapping actor
    let mut actor_mappings = ActorMapping::load(&datadir, actor_mappings_file)?;

    let mut tasks = ProcessingQueue::new_from_directory(inputdir, force_rename)?;

    while let Some(task) = tasks.take_waiting() {
        info!("id {:>10}: file: {}", task.lineid(), task.audiofile());

        // try to resolve this actorname to predefined set of available actors
        // and keep the information to store this updated alias mapping
        if let Some(actor) = processor.strings().get_actor(task.lineid()) {
            actor_mappings.resolve(actor);
        }

        tasks.update_taskresult(processor.process(task))?;
    }

    // TODO: skipped
    // info!("finished generation of #{} phoneme files.", tasks.processed());
    // if tasks.failed() > 0 {
    //     Err(format!("failed for #{} strings. see log for details.", tasks.failed()))
    // }

    if let Some(actor_mappings_file) = actor_mappings_file {
        store_actor_mappings(actor_mappings_file, &mut actor_mappings)?;
    }
    Ok(())
}
// ----------------------------------------------------------------------------
fn log_missing_audio(
    inputdir: PathBuf,
    stringsfile: Option<PathBuf>,
    language: String,
) -> Result<(), String> {
    info!(
        "LOGGING MISSING AUDIO: SCANNING {} for audio",
        inputdir.display()
    );

    let stringsfile =
        stringsfile.map_or_else(|| w3phonemetools::search_strings_file(&inputdir), Ok)?;

    let missing_data =
        w3phonemetools::find_missing_audio(inputdir, stringsfile.clone(), Some(&language))?;

    if !missing_data.is_empty() {
        use w3phonemetools::CsvWriter;

        let input_csv = stringsfile
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let infoline = format!(" missing audio files for {}", input_csv);
        let target_filename = format!("{}-missing", input_csv);

        let mut missingdata_file = stringsfile;
        missingdata_file.set_file_name(target_filename);

        let mut writer = w3phonemetools::SimpleCsvWriter::create(&missingdata_file)?;
        writer.write_comment("");
        writer.write_comment(&infoline);
        writer.write_comment("");

        for (lineid, (text, actor)) in &missing_data {
            writer.writeln(&format!(
                "{:0>10}|{}|{}",
                lineid,
                actor.as_ref().unwrap_or(&String::default()),
                text
            ));
        }

        info!(
            "stored lines with missing audio in {}",
            missingdata_file.display()
        );
    }

    Ok(())
}
// ----------------------------------------------------------------------------
fn generate_phonemes(
    stringsfile: PathBuf,
    datadir: PathBuf,
    language: String,
    actor_mappings_file: Option<&Path>,
    mut outputdir: Option<PathBuf>,
) -> Result<(), String> {
    info!(
        "GENERATING PHONEMES from strings file {}",
        stringsfile.display()
    );

    let outputdir = match outputdir.take() {
        Some(dir) => dir,
        None => {
            let mut dir = stringsfile.clone();
            dir.pop();
            dir
        }
    };
    let (generator, strings) =
        w3phonemetools::init_phoneme_generation(&language, &stringsfile, &datadir, &outputdir)?;

    // init mapping actor
    let mut actor_mapping = ActorMapping::load(&datadir, actor_mappings_file)?;

    let mut generated = 0;
    let mut failed = 0;
    let mut skipped_no_actor = 0;
    for (id, (text, actor)) in strings.get_all_lines().iter() {
        if let Some(actor) = actor.as_ref() {
            debug!("id {id:>10}: generating phonemes...");

            // try to resolve this actorname to predefined set of available actors
            // and keep the information to store this updated alias mapping
            actor_mapping.resolve(actor);

            match generator.generate(*id, actor, text) {
                Ok(_) => generated += 1,
                Err(why) => {
                    error!("{why} skipping id [{id:>10}]...");
                    failed += 1;
                }
            }
        } else {
            debug!("missing actor/voiceover info: skipping id [{id:>10}]...");
            skipped_no_actor += 1;
            continue;
        }
    }
    info!("finished generation of #{generated} phonemes files.");
    if failed > 0 {
        warn!("failed for #{failed} strings. see log for details.");
    }
    if skipped_no_actor > 0 {
        warn!("skipped #{skipped_no_actor} strings without actor info. see log for details.");
    }

    if let Some(actor_mappings_file) = actor_mappings_file {
        store_actor_mappings(actor_mappings_file, &mut actor_mapping)?;
    }
    Ok(())
}
// ----------------------------------------------------------------------------
fn store_actor_mappings(path: &Path, mappings: &mut ActorMapping) -> Result<(), String> {
    info!(
        "updating actor mapping based on processed lines and extracted actor names in: {}",
        path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or("failed to extract filename".into())
    );
    mappings.store_updated(path)
        .map_err(|err| format!("failed to store actor mapping: {err}"))
}
// ----------------------------------------------------------------------------
fn interactive_mode(
    inputdir: Option<PathBuf>,
    stringsfile: Option<PathBuf>,
    actor_mappings_file: Option<PathBuf>,
    datadir: PathBuf,
    workerthreads: usize,
    language: String,
    loglevel: LevelFilter,
) -> Result<(), String> {
    let app_name = format!("{} v{}", NAME, VERSION.unwrap_or("unknown"));

    info!("started in INTERACTIVE MODE");

    // start main gui loop
    gui::run(
        app_name,
        &PathBuf::from(HELPFILE),
        inputdir,
        language,
        stringsfile,
        actor_mappings_file,
        datadir,
        workerthreads,
        loglevel,
    )
}
// ----------------------------------------------------------------------------
fn start_main() -> Result<(), i32> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let opts = setup_option();

    println!(
        "{} v{}\n{}\n",
        NAME,
        VERSION.unwrap_or("unknown"),
        LIBRARIES
    );

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            logger::pre_init_fatal(f.to_string());
            print_usage(&program, opts);
            return Err(1);
        }
    };

    // no free args
    if matches.opt_present("h") || !matches.free.is_empty() {
        print_usage(&program, opts);
        return Ok(());
    }

    match parse_arguments(matches) {
        Ok(args) => {
            match args.mode {
                OpMode::Extract => extract_phonemes(
                    args.input.expect("input dir missing"),
                    args.strings_file,
                    args.datadir,
                    args.language,
                    args.mappings_file.as_deref(),
                    args.force_rename,
                    args.loglevel,
                ),
                OpMode::LogMissing => log_missing_audio(
                    args.input.expect("audio dir missing"),
                    args.strings_file,
                    args.language,
                ),
                OpMode::Generate => generate_phonemes(
                    args.strings_file.expect("strings-file missing"),
                    args.datadir,
                    args.language,
                    args.mappings_file.as_deref(),
                    args.outdir,
                ),
                OpMode::Interactive => interactive_mode(
                    args.input,
                    args.strings_file,
                    args.mappings_file,
                    args.datadir,
                    args.worker.unwrap_or(1),
                    args.language,
                    args.loglevel,
                ),
            }
            .map_err(|errmsg| {
                error!("{}", errmsg);
                1
            })
        }
        Err(msg) => {
            error!("{}", msg);
            print_usage(&program, opts);
            Err(1)
        }
    }
}
// ----------------------------------------------------------------------------
use std::process;
fn main() {
    let resultcode = match start_main() {
        Ok(_) => 0,
        Err(err) => err,
    };

    process::exit(resultcode);
}
// ----------------------------------------------------------------------------
