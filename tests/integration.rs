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

    fn assert_has_code(source: &str, code: DiagnosticCode) {
        let analysis = achitekfile::analyze(source).expect("expected analysis to run");

        assert!(analysis.has_errors());
        assert!(
            analysis
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code().as_str() == code.as_str()),
            "expected diagnostic code {} in {:?}",
            code.as_str(),
            analysis
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.code().as_str())
                .collect::<Vec<_>>()
        );
    }

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

    #[test]
    fn reports_unterminated_string_for_missing_closing_quote() {
        let source = indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app
            }
        "#};

        assert_has_code(source, DiagnosticCode::UnterminatedString);
    }

    #[test]
    fn reports_malformed_array_for_missing_comma() {
        let source = indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app"
            }

            prompt "kind" {
              type = select
              choices = ["bin" "lib"]
            }
        "#};

        assert_has_code(source, DiagnosticCode::MalformedArray);
    }
}
