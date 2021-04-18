use glium::{backend::Facade, Program};
use serde_json::Value;

const DEFAULT_VERTEX_SHADER: &str = include_str!("defaults/vs.glsl");
const DEFAULT_FRAGMENT_SHADER: &str = include_str!("defaults/fs.glsl");

#[derive(Debug)]
pub struct Stage {
    pub prog: Program,
    pub target: Option<String>,
}

impl Stage {
    pub fn new(prog: Program, target: Option<String>) -> Self {
        Self { prog, target }
    }

    pub fn from_json<F: Facade>(facade: &F, object: Value) -> Self {
        let fs = match object.get("fs") {
            Some(Value::String(s)) => std::fs::read_to_string(s).expect("could not read file"),
            None => DEFAULT_FRAGMENT_SHADER.to_string(),
            s => panic!("expected string, got {:?}", s),
        };

        let vs = match object.get("vs") {
            Some(Value::String(s)) => std::fs::read_to_string(s).expect("could not read file"),
            None => DEFAULT_VERTEX_SHADER.to_string(),
            s => panic!("expected string, got {:?}", s),
        };

        let target = match object.get("TARGET") {
            Some(Value::String(s)) => Some(s.clone()),
            None => None,
            s => panic!("expected string, got {:?}", s),
        };

        let prog = Program::from_source(facade, &vs, &fs, None).unwrap();
        Stage::new(prog, target)
    }
}

#[derive(Debug)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
}

impl Pipeline {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn from_json<F: Facade>(object: Value, facade: &F) -> Self {
        let passes = match object.get("PASSES") {
            Some(serde_json::Value::Array(s)) => s,
            s => panic!("expected array, got {:?}", s),
        }
        .clone();

        let stages = passes
            .into_iter()
            .map(|pass| Stage::from_json(facade, pass))
            .collect::<Vec<_>>();

        Self { stages }
    }
}
