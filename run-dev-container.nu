let dev_dir = pwd

let os_name = $nu.os-info.name

let nvim_config_dir = if $os_name == "windows" {
    $env.APPDATA | (path parse).parent | path join "local" "nvim"
} else {
    # Default to Linux/macOS config path
    "$nu.env.HOMEPATH/.config/nvim"
}

echo $"Detected OS: ($os_name)"
echo $"Neovim config directory: ($nvim_config_dir)"

let dev_mount = $"($dev_dir):/usr/src/dev"
let nvim_mount = $"($nvim_config_dir):/root/.config/nvim"

docker build -t seg-dev -f Dockerfile.dev .
docker run -it --rm -v $dev_mount -v $nvim_mount seg-dev
