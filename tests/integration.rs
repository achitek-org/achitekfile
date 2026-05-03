#[cfg(test)]
mod from_str {
    use indoc::indoc;

    #[test]
    fn happy_path() {
        let source = indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app"
            }

            prompt "project" {
              type = string
            }
        "#};

        let tree = achitekfile::from_str(source).expect("expected source to parse");
        let root = tree.root_node();

        assert_eq!(root.kind(), "file");
        assert!(!root.has_error());
    }
}

#[cfg(test)]
mod strict_analysis {
    #[test]
    fn happy_path() {
        //
    }
}

#[cfg(test)]
mod forgiving_analysis {
    use achitekfile::{DiagnosticCode, Severity};
    use indoc::indoc;

    #[test]
    fn happy_path() {
        let source = indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app"
            }

            prompt "project" {
              type = string
            }
        "#};

        let analysis = achitekfile::analyze(source).expect("expected analysis to run");

        assert_eq!(analysis.source(), source);
        assert!(!analysis.has_errors());
        assert!(analysis.diagnostics().is_empty());
    }

    #[test]
    fn reports_syntax_diagnostics_for_invalid_source() {
        let source = indoc! {r#"
            prompt "project" {
              type = string
            }
        "#};

        let analysis = achitekfile::analyze(source).expect("expected analysis to run");
        let diagnostics = analysis.diagnostics();

        assert!(analysis.has_errors());
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code().as_str() == DiagnosticCode::MissingBlueprintBlock.as_str()
                && diagnostic.severity() == Severity::Error
                && diagnostic.message() == DiagnosticCode::MissingBlueprintBlock.message()
                && diagnostic.help() == DiagnosticCode::MissingBlueprintBlock.help()
        }));
    }
}
