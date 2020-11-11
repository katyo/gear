mod artifact;
mod builder;

pub use artifact::*;
pub use builder::*;

use rhai::Engine;

#[async_std::main]
async fn main() {
    let mut engine = Engine::new();

    Artifact::register(&mut engine);
    Artifacts::register(&mut engine);
    Builder::register(&mut engine);
    //BuilderInstance::register(&mut engine);

    engine.eval_file::<()>("Genefile".into()).unwrap();
}
