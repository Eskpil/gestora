# Gestora

Gestora adds the nice macOS three finger swipe to sway. It utilizes libinput and the sway ipc protocol. 

- Three fingers to the left, changes the workspace one to the left.
- Three fingers to the right, changes the workspace one to the right.

Super simple

## Dependencies

Most likely you will need the development packages of libinput
to build gestora.

Fedora:

```shell
dnf install libinput-devel
```

Ubuntu:

```shell
apt-get install libinput-dev
```

## Install

Running make install will install Gestora on your system.

## Running with sway

Simply adding a 

```shell
exec gestora
```

to your sway config will make gestora launch together with sway.

