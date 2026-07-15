use leptos::prelude::*;

/// A decision model — a document with rules in it.
#[component]
fn ModelIcon() -> impl IntoView {
    view! {
        <svg class="file-icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
            <path d="M3.5 1h6L13 4.5V15H3.5z" fill="none" stroke="currentColor" stroke-width="1.2"/>
            <path d="M9.25 1.25V4.75H12.75" fill="none" stroke="currentColor" stroke-width="1.2"/>
            <path d="M5.5 8h5M5.5 10.5h5" stroke="currentColor" stroke-width="1.2"/>
        </svg>
    }
}

/// A dataset — rows and columns.
#[component]
fn DataIcon() -> impl IntoView {
    view! {
        <svg class="file-icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
            <rect
                x="1.6"
                y="2.6"
                width="12.8"
                height="10.8"
                rx="1"
                fill="none"
                stroke="currentColor"
                stroke-width="1.2"
            />
            <path d="M1.6 6.2h12.8M6.2 6.2v7.2" stroke="currentColor" stroke-width="1.2"/>
        </svg>
    }
}

/// What a sample file is — a DMN model or a dataset. The icon and the accessible
/// name both follow from it.
#[derive(Clone, Copy)]
pub enum FileKind {
    Model,
    Dataset,
}

impl FileKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Model => "DMN model",
            Self::Dataset => "Dataset",
        }
    }

    /// A `.dmn` file is a model; anything else here is a dataset. Used where the
    /// kind is not stated explicitly, such as an article's file list.
    pub fn from_filename(name: &str) -> Self {
        if std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dmn"))
        {
            Self::Model
        } else {
            Self::Dataset
        }
    }
}

/// One downloadable file: an icon and its name, as a bordered list item.
///
/// The icon is decorative — the filename is the link text — so it is hidden from
/// assistive technology rather than announced. The accessible name says what
/// activating the link does, which a bare filename would not.
#[component]
pub fn Download(#[prop(into)] file: String, kind: FileKind) -> impl IntoView {
    let href = format!("/examples/{file}");
    let label = format!("Download {file} ({})", kind.label());
    view! {
        <li class="download">
            <a href=href download aria-label=label>
                {match kind {
                    FileKind::Model => view! { <ModelIcon/> }.into_any(),
                    FileKind::Dataset => view! { <DataIcon/> }.into_any(),
                }}
                <span>{file}</span>
            </a>
        </li>
    }
}
