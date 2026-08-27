#[cfg(debug_assertions)]
pub(crate) const APPLICATION_ID: &str = "io.github.z99natza.zter.Devel";
#[cfg(not(debug_assertions))]
pub(crate) const APPLICATION_ID: &str = "io.github.z99natza.zter";

#[cfg(debug_assertions)]
pub(crate) const APPLICATION_NAME: &str = "zter (Development)";
#[cfg(not(debug_assertions))]
pub(crate) const APPLICATION_NAME: &str = "zter";

#[cfg(debug_assertions)]
pub(crate) const ICON_NAME: &str = "io.github.z99natza.zter.Devel";
#[cfg(not(debug_assertions))]
pub(crate) const ICON_NAME: &str = "io.github.z99natza.zter";

#[cfg(debug_assertions)]
pub(crate) const SETTINGS_DIRECTORY: &str = "zter-devel";
#[cfg(not(debug_assertions))]
pub(crate) const SETTINGS_DIRECTORY: &str = "zter";

pub(crate) const SETTINGS_RELOAD_ACTION: &str = "settings-reload";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_identity_matches_the_active_profile() {
        #[cfg(debug_assertions)]
        assert_eq!(
            (
                APPLICATION_ID,
                APPLICATION_NAME,
                ICON_NAME,
                SETTINGS_DIRECTORY,
            ),
            (
                "io.github.z99natza.zter.Devel",
                "zter (Development)",
                "io.github.z99natza.zter.Devel",
                "zter-devel",
            )
        );

        #[cfg(not(debug_assertions))]
        assert_eq!(
            (
                APPLICATION_ID,
                APPLICATION_NAME,
                ICON_NAME,
                SETTINGS_DIRECTORY,
            ),
            (
                "io.github.z99natza.zter",
                "zter",
                "io.github.z99natza.zter",
                "zter",
            )
        );
    }
}
