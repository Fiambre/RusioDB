fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile()
            .expect("no se pudo embeber el ícono en el ejecutable");
    }
}
