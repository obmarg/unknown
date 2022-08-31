#[derive(knuffel::Decode, Debug)]
pub struct TaskBlock {
    #[knuffel(children(name = "import_dir"), unwrap(argument))]
    dir_imports: Vec<String>,

    #[knuffel(children(name = "import_file"), unwrap(argument))]
    file_imports: Vec<String>,

    #[knuffel(children(name = "import_template"), unwrap(argument))]
    template_imports: Vec<String>,

    #[knuffel(children(name = "task"))]
    tasks: Vec<TaskDefinition>,
}

#[derive(knuffel::Decode, Debug)]
struct TaskDefinition {
    #[knuffel(argument)]
    name: String,

    #[knuffel(children(name = "command"), unwrap(argument))]
    commands: Vec<String>,

    #[knuffel(children(name = "dependency"))]
    dependency: Vec<TaskDependency>,

    #[knuffel(children(name = "inputs"))]
    input_blocks: Vec<InputBlock>,
}

#[derive(knuffel::Decode, Debug)]
struct TaskDependency {
    #[knuffel(property)]
    task: String,

    #[knuffel(property)]
    target: String,
    // TODO: Stuff goes here..
}

#[derive(knuffel::Decode, Debug)]
struct InputBlock {
    #[knuffel(children(name = "file"), unwrap(argument))]
    files: Vec<String>,

    #[knuffel(children(name = "dir"))]
    dirs: Vec<DirInput>,

    #[knuffel(children(name = "env_var"), unwrap(argument))]
    env_vars: Vec<String>,

    #[knuffel(children(name = "command_output"), unwrap(argument))]
    command_outputs: Vec<String>,
    // TODO: Stuff goes here..
}

#[derive(knuffel::Decode, Debug)]
struct DirInput {
    #[knuffel(argument)]
    path: String,

    #[knuffel(property)]
    glob: Option<String>,
}
