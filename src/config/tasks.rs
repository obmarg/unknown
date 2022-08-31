#[derive(knuffel::Decode, Debug)]
pub struct TaskBlock {
    #[knuffel(children(name = "import_dir"))]
    dir_imports: Vec<DirectoryImport>,

    #[knuffel(children(name = "import_file"))]
    file_imports: Vec<FileImport>,

    #[knuffel(children(name = "import_template"))]
    template_imports: Vec<TemplateImport>,

    #[knuffel(children(name = "task"))]
    tasks: Vec<TaskDefinition>,
}

#[derive(knuffel::Decode, Debug)]
struct DirectoryImport {
    #[knuffel(argument)]
    path: String,
}

#[derive(knuffel::Decode, Debug)]
struct FileImport {
    #[knuffel(argument)]
    path: String,
}

#[derive(knuffel::Decode, Debug)]
struct TemplateImport {
    #[knuffel(argument)]
    template_name: String,
}

#[derive(knuffel::Decode, Debug)]
struct TaskDefinition {
    #[knuffel(argument)]
    name: String,

    #[knuffel(children(name = "command"))]
    commands: Vec<Command>,

    #[knuffel(children(name = "dependency"))]
    dependency: Vec<TaskDependency>,

    #[knuffel(children(name = "inputs"))]
    input_blocks: Vec<InputBlock>,
}

#[derive(knuffel::Decode, Debug)]
struct Command {
    #[knuffel(argument)]
    command: String,
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
    #[knuffel(children(name = "file"))]
    files: Vec<FileInput>,

    #[knuffel(children(name = "dir"))]
    dirs: Vec<DirInput>,

    #[knuffel(children(name = "env_var"))]
    env_vars: Vec<EnvVarInput>,

    #[knuffel(children(name = "command_output"))]
    command_outputs: Vec<CommandOutputInput>,
    // TODO: Stuff goes here..
}

#[derive(knuffel::Decode, Debug)]
struct FileInput {
    #[knuffel(argument)]
    path: String,
}

#[derive(knuffel::Decode, Debug)]
struct DirInput {
    #[knuffel(argument)]
    path: String,

    #[knuffel(property)]
    glob: Option<String>,
}

#[derive(knuffel::Decode, Debug)]
struct EnvVarInput {
    #[knuffel(argument)]
    name: String,
}

#[derive(knuffel::Decode, Debug)]
struct CommandOutputInput {
    #[knuffel(argument)]
    command: String,
}
