

## Running
### Docker
#### Scanner mode
```
docker run --rm -v $PWD:/out -w /out --net host --cap-add=NET_ADMIN --cap-add NET_RAW seg scan --targets /out/target.txt --protocol both
```

#### Listener mode
```
docker run -it --rm -v $PWD:/out -w /out --net host --cap-add=NET_ADMIN --cap-add NET_RAW seg listen --protocol both
```

## Testing with Vagrant

### Hyper-v

`vagrant/hyper-v/`

From admin powershell shell

```
cd vagrant/hyper-v
$env:VAGRANT_VM_HOSTNAME="listener OR scanner"; vagrant up
```
