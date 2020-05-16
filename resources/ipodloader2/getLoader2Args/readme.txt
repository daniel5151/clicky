About "getLoader2Args"
-----------------------

	Author: Thomas Tempelmann (http://ipodlinux.org/User:Tempel)
	Last change: 01Apr06

Overview
---------

getLoader2Args is a tool that runs on the iPod under iPodLinux.

It is used in conjunction with iPodLoader2 (http://ipodlinux.org/Loader_2)

It gives you the ability to have multiple choices to launch the same
linux kernel with different options for the "userland".

Details
--------

As of 30Mar06, Loader2 allows to include arguments (text) for the linux
kernel it launches. These arguments can then be read by getLoader2Args.

To use this feature, you need a use a configuration file (see the docs
for loader2) and add your arguments as text behind the image file name,
separated by a blank (space, not TAB!).

Example:

  iPodLinux @ (hd0,1)/kernel.bin this is the arg text

Place the file "getLoader2Args" onto your linux file system (the ext2
or hfs partition), e.g. into /sbin

When the linux kernel has started, you can put this line into the
/etc/rc file to see these arguments:

  /sbin/getLoader2Args

When it executes, it should print the text "this is the arg text"
to the console (i.e. the ipod screen).

If you know your way around with linux, you should be able to do
some more useful with this, e.g. use a shell script that checks the
arg and then either launches an application or does other things.

Practical example
------------------

Note: the following assumes that you have the improved "minix" shell
installed and not only the simple "sash" shell. How to tell the
difference? Well, if the code below leads just to syntax error messages,
you got the wrong one installed. See http://ipodlinux.org/Minix-sh
for how to install the Minix shell.

Change your /etc/rc file by removing the last line reading "podzilla"
and add instead these lines:

	if [ -f /bin/getLoader2Args ] ; then
	  args=`/bin/getLoader2Args`
	  echo "Args: $args"
	fi
	if [ "$args" = "" ]; then
	  podzilla
	else
	  eval $args
	fi

Now, you can define a shell command in the loader2 configuration
file and it will be executed instead of podzilla. E.g, you could
have now these lines inside your config file:

  PodZilla @ (hd0,1)/kernel.bin     podzilla
  Linux shell @ (hd0,1)/kernel.bin  cat /proc/meminfo

Choosing the first at boot till launch podzilla, while the other
one will show the memory (RAM) info of your Linux OS.

EOT
