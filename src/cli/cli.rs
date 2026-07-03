use std::{env, ffi::OsStr};

use super::CommandLineProgram;

#[derive(Clone, Debug)]
pub struct CommandLine {
    pub args: Vec<String>,
    pub fps: f32,
    pub zoom: f32,
    pub debug: bool,
    pub bitmap: bool,
    pub adblock: bool,
    pub no_images: bool,
    pub graphics: bool,
    pub vim: bool,
    pub program: CommandLineProgram,
    pub shell_mode: bool,
}

pub enum EnvVar {
    Debug,
    Bitmap,
    Adblock,
    NoImages,
    Graphics,
    ShellMode,
    Vim,
}

impl EnvVar {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnvVar::Debug => "CARBONYL_ENV_DEBUG",
            EnvVar::Bitmap => "CARBONYL_ENV_BITMAP",
            EnvVar::Adblock => "CARBONYL_ENV_ADBLOCK",
            EnvVar::NoImages => "CARBONYL_ENV_NO_IMAGES",
            EnvVar::Graphics => "CARBONYL_ENV_GRAPHICS",
            EnvVar::ShellMode => "CARBONYL_ENV_SHELL_MODE",
            EnvVar::Vim => "CARBONYL_ENV_VIM",
        }
    }
}

impl AsRef<OsStr> for EnvVar {
    fn as_ref(&self) -> &OsStr {
        self.as_str().as_ref()
    }
}

impl CommandLine {
    pub fn parse() -> CommandLine {
        let mut fps = 60.0;
        let mut zoom = 1.0;
        let mut debug = false;
        let mut bitmap = false;
        let mut adblock = false;
        let mut no_images = false;
        let mut graphics = false;
        let mut vim = false;
        let mut shell_mode = false;
        let mut program = CommandLineProgram::Main;
        let args = env::args().skip(1).collect::<Vec<String>>();

        for arg in &args {
            let split: Vec<&str> = arg.split("=").collect();
            let default = arg.as_str();
            let (key, value) = (split.get(0).unwrap_or(&default), split.get(1));

            macro_rules! set {
                ($var:ident, $enum:ident) => {{
                    $var = true;

                    env::set_var(EnvVar::$enum, "1");
                }};
            }

            macro_rules! set_f32 {
                ($var:ident = $expr:expr) => {{
                    if let Some(value) = value {
                        if let Some(value) = value.parse::<f32>().ok() {
                            $var = {
                                let $var = value;

                                $expr
                            };
                        }
                    }
                }};
            }

            match *key {
                "-f" | "--fps" => set_f32!(fps = fps),
                "-z" | "--zoom" => set_f32!(zoom = zoom / 100.0),
                "-d" | "--debug" => set!(debug, Debug),
                "-b" | "--bitmap" => set!(bitmap, Bitmap),
                "--adblock" => set!(adblock, Adblock),
                "--no-images" => set!(no_images, NoImages),
                "-g" | "--graphics" => set!(graphics, Graphics),
                "--vim" => set!(vim, Vim),

                "-h" | "--help" => program = CommandLineProgram::Help,
                "-v" | "--version" => program = CommandLineProgram::Version,
                _ => (),
            }
        }

        if env::var(EnvVar::Debug).is_ok() {
            debug = true;
        }

        if env::var(EnvVar::Bitmap).is_ok() {
            bitmap = true;
        }

        if env::var(EnvVar::Adblock).is_ok() {
            adblock = true;
        }

        if env::var(EnvVar::NoImages).is_ok() {
            no_images = true;
        }

        if env::var(EnvVar::Graphics).is_ok() {
            graphics = true;
        }

        if env::var(EnvVar::ShellMode).is_ok() {
            shell_mode = true;
        }

        if env::var(EnvVar::Vim).is_ok() {
            vim = true;
        }

        // Graphics mode needs the page text rasterized into the framebuffer
        // (bitmap mode) — except combined with vim, where text must arrive as
        // cell runs so hints/find have something to scan; the image is then
        // placed under the text (hybrid). Explicit --bitmap still forces the
        // full-bitmap image, at the cost of hints/find.
        if graphics && !vim {
            bitmap = true;
            env::set_var(EnvVar::Bitmap, "1");
        }

        CommandLine {
            args,
            fps,
            zoom,
            debug,
            bitmap,
            adblock,
            no_images,
            graphics,
            vim,
            program,
            shell_mode,
        }
    }
}
