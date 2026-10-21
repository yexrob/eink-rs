//! View pieces shared by pages and commands.

use eink_rs::ui::{Alert, Badge, Palette, ProgressBar, Spinner, StatusMessage, Variant, use_theme};
use ink_rs::{Color, Element, FlexDirection, Props, Scope, Styled, Text, View};

use crate::backend::{Deployment, DeploymentState, Health, Level, LogLine};

/// A view that stacks its children.
pub(crate) fn column() -> View {
    View::new().flex_direction(FlexDirection::Column)
}

/// Console colors derived from the nearest [`Palette`], read with [`use_theme`].
#[derive(Clone)]
pub(crate) struct ConsoleTheme {
    pub(crate) heading: Color,
    pub(crate) muted: Color,
    pub(crate) good: Color,
    pub(crate) warning: Color,
    pub(crate) error: Color,
}

impl ConsoleTheme {
    pub(crate) fn health(&self, health: Health) -> Color {
        match health {
            Health::Healthy => self.good,
            Health::Degraded => self.warning,
            Health::Offline => self.error,
        }
    }
}

impl From<&Palette> for ConsoleTheme {
    fn from(p: &Palette) -> Self {
        Self {
            heading: p.primary,
            muted: p.info,
            good: p.success,
            warning: p.warning,
            error: p.error,
        }
    }
}

/// A Badge colored by health.
#[derive(Clone, PartialEq, Props)]
#[props(component = health_badge)]
pub(crate) struct HealthBadge {
    pub(crate) health: Health,
}

fn health_badge(cx: &mut Scope, props: &HealthBadge) -> Element {
    let theme = use_theme::<ConsoleTheme>(cx);
    Badge::new(props.health.label())
        .color(Some(theme.health(props.health)))
        .into()
}

pub(crate) fn message(variant: Variant, text: impl Into<String>) -> Element {
    StatusMessage::new(variant)
        .children(vec![Text::new(text.into())])
        .into()
}

pub(crate) fn alert(text: impl Into<String>) -> Element {
    Alert::new(Variant::Error)
        .children(vec![Text::new(text.into())])
        .into()
}

pub(crate) fn result_message(result: &Result<(), String>, success: &str) -> Element {
    match result {
        Ok(()) => message(Variant::Success, success),
        Err(error) => message(Variant::Error, error),
    }
}

pub(crate) fn meter(label: &str, value: f64) -> Element {
    column()
        .child(Text::new(format!("{label}: {value:.0}%")))
        .child(ProgressBar::default().value(value))
        .into()
}

pub(crate) fn deployment_label(deployment: &Deployment) -> String {
    let phase = match &deployment.state {
        DeploymentState::Running(phase) => phase.as_str(),
        DeploymentState::Succeeded => "succeeded",
        DeploymentState::Failed(_) => "failed",
    };
    format!(
        "#{} {} {} | {} | {:.0}%",
        deployment.id, deployment.service, deployment.version, phase, deployment.progress
    )
}

pub(crate) fn deployment_progress(deployment: &Deployment) -> Element {
    column()
        .child(Text::new(deployment_label(deployment)))
        .child(ProgressBar::default().value(deployment.progress))
        .child(match &deployment.state {
            DeploymentState::Running(phase) => Spinner::default().label(phase.clone()).into(),
            DeploymentState::Succeeded => message(Variant::Success, "Deployment complete"),
            DeploymentState::Failed(error) => message(Variant::Error, error),
        })
        .into()
}

pub(crate) fn log_text(line: &LogLine) -> String {
    let level = match line.level {
        Level::Info => "INFO",
        Level::Warn => "WARN",
        Level::Error => "ERROR",
    };
    format!("{:04}s {level:<5} {}", line.second, line.message)
}

pub(crate) fn log_line(line: &LogLine, theme: &ConsoleTheme, columns: usize) -> Element {
    let text: String = log_text(line).chars().take(columns).collect();
    Text::new(text)
        .color(match line.level {
            Level::Info => theme.muted,
            Level::Warn => theme.warning,
            Level::Error => theme.error,
        })
        .into()
}
