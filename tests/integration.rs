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
mod analysis {
    use achitekfile::model::{Dependency, PromptType, Value};

    #[test]
    fn forgiving_api() {
        let source = indoc::indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app"
            }

            prompt "project" {
              type = string
              help = "Name of the project"
              default = "demo"
              required = true
            }

            prompt "author" {
              type = string
              help = "Author of project"
              depends_on = project
            }
        "#};
        let analysis = achitekfile::analyze(source).expect("expected analysis to succeed");

        let file = analysis.file();
        let blueprint = file.blueprint();

        assert_eq!(analysis.source(), source);
        assert_eq!(
            blueprint
                .version
                .as_ref()
                .map(|version| version.value.as_str()),
            Some("1.0.0")
        );
        assert_eq!(
            blueprint.name.as_ref().map(|name| name.value.as_str()),
            Some("web-app")
        );
        assert_eq!(blueprint.description, None);
        assert_eq!(blueprint.author, None);
        assert_eq!(blueprint.min_achitek_version, None);
        let prompts = file.prompts();
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0].value.name, "project");
        assert_eq!(prompts[0].value.prompt_type, PromptType::String);
        assert_eq!(
            prompts[0].value.help.as_deref(),
            Some("Name of the project")
        );
        assert_eq!(
            prompts[0].value.default,
            Some(Value::String("demo".to_owned()))
        );
        assert_eq!(prompts[0].value.required, Some(true));
        assert_eq!(prompts[1].value.name, "author");
        assert_eq!(prompts[1].value.prompt_type, PromptType::String);
        assert_eq!(prompts[1].value.help.as_deref(), Some("Author of project"));
        assert_eq!(
            prompts[1].value.depends_on,
            Some(Dependency::Reference("project".to_owned()))
        );
        assert!(!analysis.has_errors());
        assert!(analysis.diagnostics().is_empty());
    }

    #[test]
    fn strict_api() {}

    #[test]
    fn into_valid_returns_valid_model() {
        let source = indoc::indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app"
            }

            prompt "project" {
              type = string
              required = true
            }
        "#};
        let valid = achitekfile::analyze(source)
            .expect("expected analysis to succeed")
            .into_valid()
            .expect("expected valid model");

        assert_eq!(valid.blueprint().version, "1.0.0");
        assert_eq!(valid.blueprint().name, "web-app");
        assert_eq!(valid.prompts().len(), 1);
        assert_eq!(valid.prompts()[0].name, "project");
        assert_eq!(valid.prompts()[0].prompt_type, PromptType::String);
        assert!(valid.prompts()[0].required);
    }

    #[test]
    fn into_valid_returns_diagnostics_when_required_model_data_is_missing() {
        let source = indoc::indoc! {r#"
            blueprint {
              version = "1.0.0"
            }
        "#};
        let diagnostics = achitekfile::analyze(source)
            .expect("expected analysis to succeed")
            .into_valid()
            .expect_err("expected validation diagnostics");

        assert!(diagnostics.iter().any(
            |diagnostic| diagnostic.message() == "missing required blueprint `name` attribute"
        ));
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
        let prompts = analysis.file().prompts();

        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].value.name, "project");
        assert_eq!(
            prompts[0].value.prompt_type,
            achitekfile::model::PromptType::String
        );
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

#[cfg(test)]
mod strict_analysis {
    #[test]
    fn happy_path() {
        let source = indoc::indoc! {r#"
            blueprint {
              version = "1.0.0"
              name = "web-app"
            }

            prompt "project" {
              type = string
            }
        "#};

        let analysis = achitekfile::analyze(source).expect("expected analysis to run");

        let prompts = analysis.file().prompts();

        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].value.name, "project");
        assert_eq!(
            prompts[0].value.prompt_type,
            achitekfile::model::PromptType::String
        );
        assert!(!analysis.has_errors());
    }
}
