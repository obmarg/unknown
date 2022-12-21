use std::collections::BTreeMap;

use crate::config::ValidPath;

use super::ProjectRef;

/// Each project might also contain other projects.  This function calculates
/// which paths to exclude from each project based on the set of paths for each project.
pub fn calculate_exclusions<'a>(
    input: impl Iterator<Item = (ProjectRef, &'a ValidPath)>,
) -> BTreeMap<ProjectRef, Vec<ValidPath>> {
    let mut project_paths = input.collect::<Vec<_>>();

    // sort_by_key doesn't let you return references, so here's the ugly full form :(
    project_paths.sort_by(|(_, p1), (_, p2)| p1.cmp(p2));

    let mut exclusions = BTreeMap::new();
    let mut projects = project_paths.iter();
    while let Some((project, path)) = projects.next() {
        let subpaths = projects
            .clone()
            .take_while(|(_, other_path)| other_path.starts_with(path))
            .map(|(_, path)| *path)
            .collect::<Vec<_>>();

        // Next, filter out any nested subpaths so we end up with the smallest exclusion
        // list possible.
        let mut filtered_paths = Vec::with_capacity(subpaths.len());
        let mut subpaths = subpaths.into_iter().rev().peekable();
        while let Some(path) = subpaths.next() {
            if let Some(next) = subpaths.peek() {
                if path.starts_with(next) {
                    continue;
                }
            }
            filtered_paths.push(path.clone());
        }
        filtered_paths.reverse();

        // Ideally want to simplify these exclusions as well...

        exclusions.insert(project.clone(), filtered_paths);
    }

    exclusions
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        config::{ValidPath, WorkspaceRoot},
        workspace::ProjectRef,
    };

    use super::calculate_exclusions;

    #[test]
    fn test_calculate_exclusions() {
        let input = [
            test_pair("libs/one"),
            test_pair("libs/two"),
            test_pair("libs/one/subproject"),
            test_pair("libs/one/subproject/nested"),
            test_pair("libs/one-but-not-a-folder"),
        ];

        let exclusions = calculate_exclusions(input.iter().map(|(p, r)| (p.clone(), r)))
            .into_iter()
            .map(|(project, paths)| {
                let paths = paths
                    .into_iter()
                    .map(|p| p.as_subpath().to_owned())
                    .collect::<Vec<_>>();

                (project.as_str().to_owned(), paths)
            })
            .collect::<HashMap<_, _>>();

        assert_eq!(exclusions["libs/one"], vec!["libs/one/subproject"]);
        assert!(exclusions["libs/two"].is_empty());
        assert_eq!(
            exclusions["libs/one/subproject"],
            vec!["libs/one/subproject/nested"]
        );
        assert!(exclusions["libs/one/subproject/nested"].is_empty());
        assert!(exclusions["libs/one-but-not-a-folder"].is_empty());
    }

    fn test_pair(path: &str) -> (ProjectRef, ValidPath) {
        let path = ValidPath::new_for_tests(&WorkspaceRoot::new("/Users/naebody/src"), path);
        (ProjectRef(path.clone()), path)
    }
}
