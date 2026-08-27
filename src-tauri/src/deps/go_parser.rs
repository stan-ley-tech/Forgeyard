use super::{Dependency, DependencyKind, Ecosystem};

/// Parses the `require` directives of a `go.mod` file, handling both the
/// single-line form and the parenthesized block form.
pub fn parse(contents: &str) -> Vec<Dependency> {
    let mut deps = Vec::new();
    let mut in_block = false;

    for raw_line in contents.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if in_block {
            if line == ")" {
                in_block = false;
            } else if let Some(dep) = parse_require_line(line) {
                deps.push(dep);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("require ") {
            let rest = rest.trim();
            if rest == "(" {
                in_block = true;
            } else if let Some(dep) = parse_require_line(rest) {
                deps.push(dep);
            }
        }
    }

    deps
}

fn strip_comment(line: &str) -> &str {
    line.find("//").map(|idx| &line[..idx]).unwrap_or(line)
}

fn parse_require_line(line: &str) -> Option<Dependency> {
    let mut parts = line.split_whitespace();
    let name = parts.next()?.to_string();
    let version_spec = parts.next()?.to_string();
    Some(Dependency {
        name,
        version_spec,
        kind: DependencyKind::Runtime,
        ecosystem: Ecosystem::Go,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_line_requires() {
        let deps = parse("module example.com/app\n\ngo 1.22\n\nrequire github.com/a/b v1.2.3\n");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "github.com/a/b");
        assert_eq!(deps[0].version_spec, "v1.2.3");
    }

    #[test]
    fn parses_block_requires_and_ignores_indirect_comments() {
        let contents = "module example.com/app\n\nrequire (\n\tgithub.com/gin-gonic/gin v1.9.1\n\tgithub.com/x/y v0.1.0 // indirect\n)\n";
        let deps = parse(contents);

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[1].name, "github.com/x/y");
        assert_eq!(deps[1].version_spec, "v0.1.0");
    }
}
