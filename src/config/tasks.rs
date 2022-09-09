#[derive(knuffel::Decode, Debug, Default)]
pub struct TaskBlock {
    #[knuffel(children(name = "import_dir"), unwrap(argument))]
    dir_imports: Vec<String>,

    #[knuffel(children(name = "import_file"), unwrap(argument))]
    file_imports: Vec<String>,

    #[knuffel(children(name = "import_template"), unwrap(argument))]
    template_imports: Vec<String>,

    #[knuffel(children(name = "task"))]
    pub tasks: Vec<TaskDefinition>,
}

#[derive(knuffel::Decode, Debug)]
pub struct TaskDefinition {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(children(name = "command"), unwrap(argument))]
    pub commands: Vec<String>,

    #[knuffel(children(name = "dependency"))]
    pub dependencies: Vec<TaskDependency>,

    #[knuffel(children(name = "inputs"))]
    input_blocks: Vec<InputBlock>,
}

#[derive(knuffel::Decode, Debug)]
pub struct TaskDependency {
    #[knuffel(property)]
    pub task: String,

    // TODO: This feels like a shit name, come up with something better.
    // for_ancestors?  for_parents?
    // run_for_parents?
    #[knuffel(property)]
    pub for_project_deps: Option<bool>,

    #[knuffel(property(name = "self"))]
    pub include_this_package: Option<bool>,
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
