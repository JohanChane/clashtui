use super::ClashBackend;

impl ClashBackend {
    #[cfg(test)]
    pub fn get_connections(&self) -> Result<(Option<Vec<Vec<String>>>, (u64, u64)), String> {
        use crate::utils::bytes_to_readable;
        Ok((
            Some(vec![
                vec![
                    "".to_string(),
                    "objects.githubusercontent.com".to_string(),
                    "DIRECT".to_string(),
                    "2024-06-30T09:20:17.386789854Z".to_string(),
                    bytes_to_readable(854),
                    bytes_to_readable(7652)
                ];
                3
            ]),
            (10000, 0),
        ))
    }
    #[cfg(not(test))]
    pub fn get_connections(&self) -> Result<(Option<Vec<Vec<String>>>, (u64, u64)), String> {
        use crate::utils::bytes_to_readable;
        use api::{Conn, ConnInfo, ConnMetaData};
        let raw = self.clash_api.connections(true, None).map_err(|e| {
            log::error!("CONN(GET):{e}");
            e
        })?;
        log::debug!("CONN(GET):RAW=>{raw}");
        let ConnInfo {
            download_total,
            upload_total,
            connections,
        } = ConnInfo::try_from(raw).expect("CONN:DeSer ERR");
        Ok((
            Some(
                connections
                    .into_iter()
                    .flat_map(|t| t.into_iter())
                    .map(|c| {
                        let Conn {
                            id: _,
                            metadata,
                            upload,
                            download,
                            start,
                            mut chains,
                        } = c;
                        assert!(!&chains.is_empty());
                        let ConnMetaData {
                            network: _,
                            ctype: _,
                            host,
                            process,
                            process_path: _,
                            source_ip: _,
                            source_port: _,
                            remote_destination: _,
                            destinatio_port: _,
                        } = metadata;
                        vec![
                            process,
                            host,
                            chains.pop().unwrap(),
                            start,
                            bytes_to_readable(upload),
                            bytes_to_readable(download),
                        ]
                    })
                    .collect(),
            ),
            (download_total, upload_total),
        ))
    }
    pub fn terminate_conn<S: AsRef<str>>(&self, id: S) -> Result<(), String> {
        if let Err(e) = self
            .clash_api
            .connections(false, Some(id.as_ref().to_string()))
        {
            log::error!("CONN(DEL BY ID):{e:?}");
            return Err(e);
        } else {
            Ok(())
        }
    }
    pub fn terminate_all_conns(&self) -> Result<(), String> {
        if let Err(e) = self.clash_api.connections(false, None) {
            log::error!("CONN(DEL):{e:?}");
            return Err(e);
        } else {
            Ok(())
        }
    }
}
