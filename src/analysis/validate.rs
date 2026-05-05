use crate::{
    Diagnostic, DiagnosticCode,
    model::{
        AchitekFile, Blueprint, Prompt, PromptType, Spanned, ValidAchitekFile, ValidBlueprint,
        ValidPrompt,
    },
};

pub(super) fn validate_file(file: AchitekFile) -> Result<ValidAchitekFile, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let blueprint = validate_blueprint(file.blueprint(), &mut diagnostics);
    let prompts = file
        .prompts()
        .iter()
        .map(validate_prompt)
        .collect::<Vec<_>>();

    if diagnostics.is_empty() {
        Ok(ValidAchitekFile::new(blueprint, prompts))
    } else {
        Err(diagnostics)
    }
}

fn validate_blueprint(blueprint: &Blueprint, diagnostics: &mut Vec<Diagnostic>) -> ValidBlueprint {
    let blueprint_range = blueprint.range.unwrap_or_default();
    let version = match &blueprint.version {
        Some(version) => version.value.clone(),
        None => {
            diagnostics.push(Diagnostic::with_message(
                DiagnosticCode::MissingBlueprintVersion,
                blueprint_range,
                "missing required blueprint `version` attribute",
            ));
            String::new()
        }
    };
    let name = match &blueprint.name {
        Some(name) => name.value.clone(),
        None => {
            diagnostics.push(Diagnostic::with_message(
                DiagnosticCode::MissingBlueprintName,
                blueprint_range,
                "missing required blueprint `name` attribute",
            ));
            String::new()
        }
    };

    ValidBlueprint {
        version,
        name,
        description: blueprint
            .description
            .as_ref()
            .map(|description| description.value.clone()),
        author: blueprint.author.as_ref().map(|author| author.value.clone()),
        min_achitek_version: blueprint
            .min_achitek_version
            .as_ref()
            .map(|version| version.value.clone()),
    }
}

fn validate_prompt(prompt: &Spanned<Prompt>) -> ValidPrompt {
    let prompt = &prompt.value;

    ValidPrompt {
        name: prompt.name.clone(),
        prompt_type: prompt.prompt_type.unwrap_or(PromptType::String),
        help: prompt.help.clone(),
        choices: prompt.choices.clone(),
        default: prompt.default.clone(),
        required: prompt.required.unwrap_or(false),
        depends_on: prompt.depends_on.clone(),
        validation: prompt.validation.clone(),
    }
}
