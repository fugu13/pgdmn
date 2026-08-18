pub mod article;
pub mod articles;
pub mod docs;
pub mod examples;
pub mod home;
pub mod not_found;
pub mod why;

use crate::routes;

/// One hand-written page: its nav label, route segment, and meta description.
pub struct Page {
    pub label: &'static str,
    pub segment: &'static str,
    pub description: &'static str,
}

/// The single list the header nav and the `llms.txt` overview both walk, so
/// the two can never disagree; a new page is added here once. Home is absent
/// on purpose—the nav writes it by hand and `llms.txt` opens with it.
pub const PAGES: &[Page] = &[
    Page {
        label: "Why pgdmn",
        segment: routes::WHY,
        description: why::DESCRIPTION,
    },
    Page {
        label: "Docs",
        segment: routes::DOCS,
        description: docs::DESCRIPTION,
    },
    Page {
        label: "Examples",
        segment: routes::EXAMPLES,
        description: examples::DESCRIPTION,
    },
    Page {
        label: "Articles",
        segment: routes::ARTICLES,
        description: articles::DESCRIPTION,
    },
];

#[cfg(test)]
mod tests {
    use super::PAGES;
    use crate::site::DESCRIPTION_LIMIT;

    /// The same contract the articles enforce for their front matter: a
    /// description a crawler truncates mid-sentence reads as neglect.
    /// `site::DESCRIPTION` is the documented exception; these are not.
    #[test]
    fn every_page_description_survives_a_crawler_intact() {
        for page in PAGES {
            assert!(
                page.description.len() <= DESCRIPTION_LIMIT,
                "{}: description is {} characters, over the {DESCRIPTION_LIMIT} a crawler shows",
                page.label,
                page.description.len(),
            );
        }
    }
}
