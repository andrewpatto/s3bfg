use crate::config::Config;
use rusoto_credential::{
    AwsCredentials, ChainProvider, DefaultCredentialsProvider, ProfileProvider,
    ProvideAwsCredentials,
};
use std::time::Duration;

pub async fn fetch_credentials(config: &Config) -> (AwsCredentials, String) {
    return if config.aws_profile.is_some() {
        let profile_name = config.aws_profile.as_ref().unwrap();

        let mut pp = ProfileProvider::new().unwrap();
        pp.set_profile(profile_name);
        let mut cp = ChainProvider::with_profile_provider(pp);
        // out expectation is to be running in AWS so this is plenty of time for it to
        // get any EC2 role credentials
        cp.set_timeout(Duration::from_millis(500));
        let creds = cp.credentials().await.unwrap();

        (
            creds.clone(),
            format!(
                "Profile `{}` -> {:?}",
                profile_name,
                creds,
            ),
        )
    } else {
        let creds = DefaultCredentialsProvider::new()
            .unwrap()
            .credentials()
            .await
            .unwrap();

        (
            creds.clone(),
            format!(
                "Default provider -> {:?}",
                creds
            ),
        )
    };
}
