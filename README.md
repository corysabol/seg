# Seg

Seg is a segmentation testing tool designed to overcome some of the challenges that typically come with segmentation scanning.
It offers two modes; `seg scan`, and `seg listen`.

## Current Features
- 🔍 **Listen Mode**
  - Greppable output format
  - Custom port forwarding rules
  
- 🎯 **Scanner Mode**
  - Relies on nmap currently
  - Greppable output for easy triage
  
- 🌐 **Protocol Support**
  - Supports TCP and UDP
 
See the [Future Development](#-future-development) section for a list of planned features.

## Scan mode
In scan mode seg will accept input from a file which contains a list of network tags (strings which identify the network to the user) followed by a listener IP address. See [Target Specification](#target-specification).
The tool will then leverage nmap or a pure rust in-built scanner depending on the supplied options to scan all ports for each listener supplied. This can be done for UDP and TCP.

## Listen mode
In listen mode, seg will leverage nftables to establish port fowarding rules, and an anti-lockout rule. These rules can be customized using flags or by supplying a custom nft ruleset.
If you need a set of base rules to work off of when creating custom rules, `seg listen` has the `--emit-rules` flag which will print the default rules used by seg to stdout. You can modify
these and pass them to a listener using the `--rules` flag.

See the [usage](#usage) section below, or the [examples](#examples) section for a more thorough walkthrough on how to use seg.

## Installation

Aside from using the provided docker image with `docker pull 84d93r/seg` you can find prebuilt binaries under [releases](https://github.com/corysabol/seg/releases).

## Usage

### Listen mode
```
Run in listener mode

Usage: seg listen [OPTIONS]

Options:
      --emit-rules
          Emits the base rules template for customization
      --rules <RULES>
          An optional rules file to use
      --protocol <PROTOCOL>
          The protocol to listen for connection over. NOT YET IMPLEMENTED! [default: both] [possible values: tcp, udp, both]
  -l, --listen-address <LISTEN_ADDRESS>
          [default: 0.0.0.0]
  -a, --access-port <ACCESS_PORT>
          Port used to access the host (typicall 22 for ssh) [default: 22]      
  -p, --port <PORT>
          Port to listen on for both TCP and UDP [default: 5555]
  -h, --help
          Print help
```

### Scan mode
```
Run in scanner mode

Usage: seg scan [OPTIONS] --input-file <INPUT_FILE>

Options:
  -i, --input-file <INPUT_FILE>  Path to the file containing lines of network-na
me,listener-ip
  -s, --scan-type <SCAN_TYPE>    [default: both] [possible values: tcp, udp, both]
  -h, --help                     Print help
```


## Target specification

For now targets are only input to scan mode via a file containing lines of the following format:
```
network-name,scanner-ip
```

## Running

### Binary

#### Scanner mode
```
seg scan --input-file networks.txt
```

#### Listener mode
```
seg listen
```

### Docker

```
docker pull 84d93r/seg
```

#### Scanner mode
```
docker run --rm -v $PWD:/out -w /out --net host --cap-add=NET_ADMIN --cap-add NET_RAW 84d93r/seg scan --targets /out/target.txt --protocol both
```

#### Listener mode
```
docker run -it --rm -v $PWD:/out -w /out --net host --cap-add=NET_ADMIN --cap-add NET_RAW 84d93r/seg listen --protocol both
```

## Examples
**TODO**

## Testing with Vagrant

### Hyper-v

`vagrant/hyper-v/`

From admin powershell shell

```
cd vagrant/hyper-v
vagrant up
vagrant ssh listener
# in another shell
vagrant ssh scanner
```
You can run the binary from these VMs to test the tool over the VM network.

## 🎯 Seg Tool Roadmap

### ✅ Currently Implemented
- 🔍 **Listen Mode**
  - Greppable output format
  - Custom port forwarding rules
  
- 🎯 **Scanner Mode**
  - Nmap integration
  - Greppable output
  
- 🌐 **Protocol Support**
  - TCP
  - UDP

### 🚀 Future Development

- 🦀 **Native Rust Port Scanner**
  - Alternative to Nmap dependency

- 📊 **Enhanced Output**
  - JSON format support
  - Structured data export
  - Machine-readable logs

- 📈 **Data Visualization Tool**
  - Separate tool for visualizing discovered connections between networks

- 🛡️ **Advanced Scanning**
  - SYN stealth scans
  - Custom scan patterns

- 🪟 **Windows Support**
  - Integration with windows firewall for port forwarding rules
  - Windows builds

---
