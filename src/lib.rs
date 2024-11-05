use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::process::Command;
use std::ptr;
use anyhow::bail;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{StatusCode, Url};
use serde_json::{json, Value};
use tokio::runtime::Runtime;

// #[no_mangle]
// pub extern "C" fn get(param: *mut c_char) -> c_int {
//     let res = CString::new("test nihao").unwrap();
//     let raw = res.as_ptr();
//     let len = res.as_bytes_with_nul().len();
//     unsafe {
//         ptr::copy_nonoverlapping(raw, param, len);
//     }
//
// }

#[no_mangle]
pub extern "C" fn Post(url: *const c_char, param: *const c_char, headers: *const c_char, dest: *mut c_char) {
    let result = match do_post(url, param, headers) {
        Ok(result) => {
            json!({
                "code": 0,
                "data": result
            })
        }
        Err(err) => {
            json!({
                "code": -1,
                "err": format!("{}", err.to_string())
            })
        }
    };

    let result = CString::new(serde_json::to_string(&result).unwrap()).unwrap();
    let source = result.as_ptr();
    let len = result.to_bytes_with_nul().len();
    unsafe {
        ptr::copy_nonoverlapping(source, dest, len);
    }
}

#[no_mangle]
pub extern "C" fn Get(url: *const c_char) -> *mut c_char {
    let result = match do_get(url) {
        Ok(data) => {
            data
        },
        Err(e) => {
            e.to_string()
        }
    };

    let result = CString::new(result).unwrap();
    result.into_raw()
}

fn do_get(url: *const c_char) -> anyhow::Result<String> {
    let rt = Runtime::new().expect("tokio runtime create error");
    // let url = unsafe { convert_to_string(url)? };
    // let mut command = Command::new("sh");
    // command.arg("-c").arg(&format!("curl -s -X GET \"{}\"", url));
    // match command.output() {
    //     Ok(output) => {
    //         let err = String::from_utf8_lossy(&output.stderr).to_string();
    //         if !err.is_empty() {
    //             bail!("url 访问失败: {}", err);
    //         }
    //         Ok(String::from_utf8_lossy(&output.stdout).to_string())
    //     }
    //     Err(e) => {
    //         bail!(format!("rust程序执行出错： {}", e))
    //     }
    // }
    rt.block_on(async move {
        if url.is_empty(){
            bail!("请求地址为空");
        }
        let req = match reqwest::get(url).await {
            Ok(resp) => resp,
            Err(e) => {
                bail!("{e:?}");
            }
        };
        Ok(req.text().await?)
    })
}

fn do_post(url: *const c_char, param: *const c_char, headers: *const c_char) -> anyhow::Result<String> {
    let rt = Runtime::new().expect("tokio runtime create error");
    let url = unsafe { convert_to_string(url)? };
    let param = unsafe { convert_to_string(param)? };
    let headers = unsafe { convert_to_string(headers)? };
    rt.block_on(async move {
        if url.is_empty() || param.is_empty() {
            bail!("请求地址或请求入参为空");
        }
        let headers = if headers.is_empty() {
            None
        } else {
            let mut headers_map = HeaderMap::new();
            let map = serde_json::from_str::<HashMap<String, Value>>(&headers)?;
            for (k, v) in map.into_iter() {
                match v {
                    Value::String(s) => {
                        headers_map.insert(
                            HeaderName::from_bytes(k.as_bytes())?,
                            HeaderValue::from_bytes(s.as_bytes())?,
                        );
                    }
                    _ => {}
                }
            }
            Some(headers_map)
        };
        do_post0(Url::parse(&url)?, headers, param).await
    })
}

async fn do_post0(url: Url, headers: Option<HeaderMap>, body: String) -> anyhow::Result<String> {
    let mut client = reqwest::Client::new().post(url);
    if let Some(headers) = headers {
        client = client.headers(headers);
    }
    let resp = client.json(&body).send().await?;
    let status = resp.status();
    let data = resp.text().await?;
    if status != StatusCode::OK && data.is_empty() {
        bail!("请求失败 错误码：{}", status.as_u16());
    }
    Ok(data)
}

unsafe fn convert_to_string(s: *const c_char) -> Result<String, std::str::Utf8Error> {
    if s.is_null() {
        Ok(String::new())
    } else {
        Ok(String::from(CStr::from_ptr(s).to_str()?))
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;
    use super::*;

    #[test]
    fn post_test() {
        let url = CString::new("http://172.16.140.130:12867/testhbp").unwrap();
        let param = CString::new("{\"a\": 1}").unwrap();
        let c: *mut c_char = unsafe { libc::malloc(1024) as *mut c_char };

        Post(url.into_raw(), param.into_raw(), ptr::null(), c);
        unsafe {
            let res = CStr::from_ptr(c).to_str().unwrap();
            println!("{res}");
        }
    }

    #[test]
    fn get_test() {
        let url = CString::new("https://blog.csdn.net/zhangwenchao0814/article/details/116718136#comments_25209169").unwrap();
        let c = Get(url.into_raw());
        unsafe {
            let res = CStr::from_ptr(c).to_str().unwrap();
            println!("{res}");
        }
    }
}