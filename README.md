## Target specification

For now targets are only input to scan mode via a file containing lines of the following format:
```
network-name,scanner-ip
```

## Running
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
