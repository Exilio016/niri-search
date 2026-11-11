use anyhow::{Result, bail};
use niri_ipc::{
    Action,
    Request::{self},
    Response, Window, Workspace,
    socket::Socket,
};
use rofi::Rofi;
use std::{
    collections::HashMap,
    fs::{create_dir, remove_dir_all},
    io::Read,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
    thread::sleep,
    time::{Duration, Instant},
};
use wl_clipboard_rs::{
    copy,
    paste::{self, ClipboardType::Regular},
};

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

fn get_tmpdir() -> Result<PathBuf> {
    let mut file = std::env::temp_dir();
    file.push("niri-search");
    if !file.exists() {
        create_dir(&file)?;
    }
    return Ok(file);
}

static FILE_COUNTER: AtomicU32 = AtomicU32::new(1);
const SCREENSHOT_TIMEOUT: Duration = Duration::from_secs(5);

fn get_window_screen(sock: &mut Socket, win: &Window) -> Result<String> {
    let mut file = get_tmpdir()?;
    file.push(format!(
        "{}.png",
        FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let path = file.to_string_lossy().into_owned();

    let action = Action::ScreenshotWindow {
        id: Some(win.id),
        write_to_disk: true,
        show_pointer: false,
        path: Some(path.clone()),
    };
    match sock.send(Request::Action(action))? {
        Ok(Response::Handled) => {}
        Ok(other) => bail!("unexpected response: {other:?}"),
        Err(msg) => bail!("{msg}"),
    }

    let deadline = Instant::now() + SCREENSHOT_TIMEOUT;
    loop {
        if file.exists() {
            return Ok(path);
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for screenshot {}", path);
        }
        sleep(Duration::from_millis(10));
    }
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

fn restore_clipboard(content: Option<Vec<u8>>, mime_type: Option<String>) -> Result<()> {
    if let Some(content) = content {
        if let fork::Fork::Child = fork::daemon(false, false)? {
            let mut opts = copy::Options::new();
            opts.foreground(true);
            let slice = content.into_boxed_slice();
            opts.copy(
                copy::Source::Bytes(slice),
                copy::MimeType::Specific(mime_type.unwrap()),
            )?;
        }
    }
    Ok(())
}

fn get_clipboad_content() -> Result<(Option<Vec<u8>>, Option<String>)> {
    let result = paste::get_contents(Regular, paste::Seat::Unspecified, paste::MimeType::Any);
    let mut content = None;
    let mut mime_type = None;
    match result {
        Ok((mut pipe, mime)) => {
            let mut data = Vec::new();
            pipe.read_to_end(&mut data)?;
            content = Some(data);
            mime_type = Some(mime);
        }
        Err(paste::Error::NoSeats | paste::Error::ClipboardEmpty | paste::Error::NoMimeType) => {}
        Err(e) => bail!(e),
    };
    return Ok((content, mime_type));
}

fn main() -> Result<()> {
    let tmp = get_tmpdir()?;
    let mut sock = Socket::connect()?;
    let workspaces = get_workspaces(&mut sock)?;
    let windows = get_all_windows(&mut sock)?;
    let mut list = Vec::new();

    let (content, mime_type) = get_clipboad_content()?;

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

    remove_dir_all(tmp)?;
    restore_clipboard(content, mime_type)?;
    Ok(())
}
