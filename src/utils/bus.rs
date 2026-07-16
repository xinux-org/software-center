use sqlx::SqlitePool;
use zbus::{
    connection,
    fdo::Error as ZError,
    interface,
    zvariant::{SerializeDict, Type},
};

use crate::{APPINFO, utils::packages::appsteamdata};

mod pkgs {
    use sqlx::SqlitePool;

    pub async fn search_initial(name: String) -> anyhow::Result<Vec<String>> {
        println!("reading $HOME");
        let home = std::env::var("HOME").unwrap();
        println!("$HOME is {:?}", home);
        let path = format!("{}/.cache/nix-data/nixpkgs.db", home,);
        let pool = &SqlitePool::connect(&format!("sqlite://{}", path)).await?;
        let packages: Vec<(String,)> = sqlx::query_as(r#"SELECT pkgs.attribute FROM pkgs JOIN meta ON pkgs.attribute = meta.attribute WHERE pkgs.attribute LIKE $1 || '%'"#)
        .bind(&name)
        .fetch_all(pool)
        .await?;

        let packages = packages
            .iter()
            .map(|(package,)| package.to_string())
            .collect::<Vec<_>>();

        Ok(packages)
    }
}

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
struct Meta {
    id: String,
    name: String,
    description: Option<String>,
    gicon: Option<String>, // path
}

struct SearchProvider {}

#[interface(name = "org.gnome.Shell.SearchProvider2")]
impl SearchProvider {
    async fn get_initial_result_set(&self, terms: Vec<String>) -> Result<Vec<String>, ZError> {
        println!("Initial results for: {:?}", terms);
        let arg = terms.join(" ");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let asdf = rt.block_on(async {
            self::pkgs::search_initial(arg)
                .await
                .map_err(|_| ZError::Failed("Can't search for packages.".to_string()))
        });

        println!("Results: {:?}", asdf);

        asdf
    }

    async fn get_subsearch_result_set(
        &self,
        _previous_results: Vec<String>,
        terms: Vec<String>,
    ) -> Result<Vec<String>, ZError> {
        println!(
            "Subsearch results for: {:?} | {:?}",
            _previous_results, terms
        );

        // let arg = previous_results.join(" ");

        self.get_initial_result_set(terms).await

        // Ok(previous_results
        //     .iter()
        //     .filter(|s| s.contains(&arg))
        //     .map(|s| s.to_string())
        //     .collect())
    }

    async fn get_result_metas(&self, identifiers: Vec<String>) -> Result<Vec<Meta>, ZError> {
        println!("Result metas for: {:?}", identifiers);

        let rt = tokio::runtime::Runtime::new().unwrap();

        let appstream_data = rt
            .block_on(async {
                appsteamdata()
                    .map_err(|_| ZError::Failed(String::from("Can't read appstream data")))
            })
            .unwrap();

        let metas = identifiers
            .iter()
            .map(|package| {
                let app_data = appstream_data.get(package);

                println!("Metas for [{package}]: {:?}", app_data);

                let mut name = String::from("There could have been your ad");
                let mut summary = String::from("There could have been your ad");
                let mut icon = None;

                if let Some(data) = app_data {
                    if let Some(i) = &data.icon
                        && let Some(mut i) = i.cached.clone()
                    {
                        i.sort_by_key(|x| x.height);
                        if let Some(i) = i.last() {
                            // icon = Some(format!(
                            //     "{}/icons/nixos/{}x{}/{}",
                            //     APPINFO, i.width, i.height, i.name
                            // ));
                            icon = Some(format!(
                                "{}/icons/nixos/{}x{}/{}",
                                "/home/dyrbk/dev/software-center/result/share/app-info",
                                i.width,
                                i.height,
                                i.name
                            ));
                        }
                    }

                    name = if let Some(n) = &data.name
                        && let Some(n) = n.get("C")
                    {
                        n.to_string()
                    } else {
                        String::from("There could have been your ad")
                    };

                    summary = if let Some(s) = &data.summary
                        && let Some(s) = s.get("C")
                    {
                        s.to_string()
                    } else {
                        String::from("There could have been your ad")
                    };
                }

                let meta = Meta {
                    id: package.to_string(),
                    name: name,
                    description: Some(summary),
                    gicon: icon,
                };

                meta
            })
            .collect::<Vec<_>>();

        println!("Returned result metas: {:?}", metas);

        Ok(metas)
    }

    async fn activate_result(&self, identifier: String, terms: Vec<String>, timestamp: u32) {
        println!(
            "Activated: {:?} for {:?} at {:?}",
            identifier, terms, timestamp
        );
    }

    async fn launch_search(&self, terms: Vec<String>, timestamp: u32) {
        println!("Launching search for: {:?} at {:?}", terms, timestamp);
    }
}

pub async fn start_search_provider() -> zbus::Result<zbus::Connection> {
    println!("Starting D-Bus...");
    let provider = SearchProvider {};

    let connection = connection::Builder::session()?
        .replace_existing_names(true)
        .allow_name_replacements(true)
        .name("uz.xinux.NixSoftwareCenter.SearchProvider")?
        .serve_at("/uz/xinux/NixSoftwareCenter/SearchProvider", provider)?
        .build()
        .await?;

    println!("D-Bus started. Pending");
    // std::future::pending::<()>().await;
    println!("D-Bus stopped(?)");

    Ok(connection)
}
