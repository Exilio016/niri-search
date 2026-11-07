use std::{collections::HashMap, sync::atomic::{AtomicU32, Ordering}};

use anyhow::{Result, bail};
use niri_ipc::{
    Action, Request::{self}, Response, Window, Workspace, socket::{Socket},
};
use rofi::Rofi;

macro_rules! unwrap_res {
    ($expr:expr, $type:pat => $ret:ident) => {{
        let res = match $expr {
            Ok(res) => res,
            Err(e) => bail!(e),
        };

        if let $type = res {
            $ret
        } else {
            bail!("Unexpected Response: {:?}", res);
        }
    }};
}

fn get_all_windows(sock: &mut Socket) -> Result<Vec<Window>> {
    let res = sock.send(niri_ipc::Request::Windows)?;
    let windows = unwrap_res!(res, Response::Windows(windows) => windows);
    Ok(windows)
}

static FILE_COUNTER: AtomicU32 = AtomicU32::new(1);

fn get_window_screen(sock: &mut Socket, win: &Window) -> Result<String> {
    let mut file = std::env::temp_dir();
    file.push(format!("{}.png", FILE_COUNTER.fetch_add(1,Ordering::Relaxed)));
    let path = file.into_string().map_err(|e| { anyhow::anyhow!("{:?}",e) })?;

    let action = Action::ScreenshotWindow { id: Some(win.id), write_to_disk: true, show_pointer: false, path: Some(path.clone())};
    let _ = sock.send(Request::Action(action))?;
    return Ok(path);
}

fn get_workspaces(sock: &mut Socket) -> Result<HashMap<u64, Workspace>> {
    let workspaces = sock.send(Request::Workspaces)?;
    let workspaces = unwrap_res!(workspaces, Response::Workspaces(output) => output);
    let mut map = HashMap::new();
    for w in workspaces {
        map.insert(w.id, w);
    }
    return Ok(map);
}

fn focus_window(sock: &mut Socket, win: &Window) -> Result<()> {
    let action = Action::FocusWindow { id: win.id };
    let _ = sock.send(Request::Action(action))?;
    Ok(())
}

fn main() -> Result<()> {
    let mut sock = Socket::connect()?;
    let workspaces = get_workspaces(&mut sock)?;
    let windows = get_all_windows(&mut sock)?;
    let mut list = Vec::new();
    for w in &windows {
        let screenshot = get_window_screen(&mut sock, w)?;
        let name = format!(
            "[{}] - {}",
            workspaces.get(&w.workspace_id.unwrap()).unwrap().idx,
            w.title.clone().unwrap(),
        );
        list.push(format!("{}\0icon\x1f{}", name, screenshot));
    }
    match Rofi::new(&list)
        .prompt("Select window")
        .theme(Some("preview"))
        .run_index()
    {
        Ok(selected) => focus_window(&mut sock, windows.get(selected).unwrap())?,
        Err(_) => {}
    }
    Ok(())
}
