# village-tui
a distributed peer to peer application implementing built-in Catalog app-type of
[gnome](https://github.com/dxtr85/gnome) protocol.

## Overview
This application is intended to provide fast and easy access to other applications
and resources created on top of gnome protocol that uses [swarm-consensus](https://github.com/dxtr85/swarm-consensus/)
mechanism for ensuring synchronization of data across the Internet.

## User interface
Every user will be given a village to maintain.
His village will consist of a street with his own home and other houses
that are owned by users' friends and others.
User will be able to enter selected home as a guest and see what's inside.
User will also be allowed to visit another user's village in order to browse
it's contents. 
Maybe we can have a house with regular front door and back door used as a teleport.
Inside his house a user will define rooms with items inside of them.
Access to each room (and house, and not village!) will be defined by the user.
A room will contain data or links to other user's data.
Others will be allowed to open/download data behind those links.
In front of every house there will be a post box where any user will be able to
put an encrypted message to owner of given house.
A user can create more streets if he wants to.
On those streets he can put other houses or different kinds of buildings
representing different types of p2p applications that he and visitors
will be able to enter and use.

## Implementation
Every village will have it's data stored in a p2p fashion by using a swarm.
If a user adds a new friend, then he will also sync that swarm onto his hard drive.
User will be able to configure what data should get synchronized by default,
or for particular friend. If someone stores a lot of files and our memory space is
limited we can sync only parts of our friends swarmed data.

This is a very high level overview and it will take a lot of time to crystalize.

## How to run in developer's mode under Linux
This entire thing is VERY experimental but functional.
I HAVE NOT tested it on a larger scale, so expect anything.
For now you will have to learn everything on your own.
Maybe someone is willing to maintain a Wiki page,
but I really do not have time for that.

- download & install rust toolchain from https://rustup.rs
- add a user with login: dxtr and log-in as that user
- mkdir /home/dxtr/projects ; cd /home/dxtr/projects
- git clone https://github.com/dxtr85/animaterm.git
- git clone https://github.com/dxtr85/village-tui.git
(or from SF: git clone git://git.code.sf.net/p/village-tui/code village-tui)
- git clone https://github.com/dxtr85/dapp-lib.git
- git clone https://github.com/dxtr85/gnome.git
- git clone https://github.com/dxtr85/swarm-consensus.git
- cd /home/dxtr/projects/village-tui
- export COLUMNS=$(tput cols)
- export LINES=$(tput lines)
- cargo run <config dir> 2> /path/to/logfile/or/dev/null
(or append following lines to /home/dxtr/.bashrc file
and later use a single letter command 'v' to run this app:
export LINES=$(tput lines)
export COLUMNS=$(tput cols)
alias v='cd /home/dxtr/projects/village-tui; cargo run ~/.village 2> log'
)

REQUIRED: You will need some neighbors defined, so under <config dir>
create neigh.conf and fill it with known neighbors like following:
# IPv4 or v6  PORT  NAT PORTALLOC TRANSPORT
192.168.0.103 62552 0 0 1

You can discover your PORT by 'less /path/to/logfile'
and searching for '- - - - -' string ...
You can keep last three numbers as above, I guess,
or dig into source code config.rs to understand it.
(Not sure if the order of last three columns is right.)

If you are willing to run a public gnome instance, lemme know,
I will post it on https://sourceforge.net/p/village-tui/wiki/Home/
 page for others to join your swarm.


### Details
In order to create a village, or any other implementation
of Catalog app type we need to define data types within 
Catalog. Then village app can interpret these types according to it's 
environment of streets, houses, rooms etc.
A house/building is a representation of a link to another application.
A link consists of SwarmID (unique FounderId and a SwarmName), and ContentID.
ContentID is just a number used to identify some particular data within a swarm.
ContentID should map to ContentHeader and DataChunks.
ContentHeader should have DataType field, name and ContentIndex.
ContentIndex should be a binary tree of hashes of sub-hashes of Leafs that are
1024 bytes long data chunks.
DataChunks, when synced, should contain all the data chunks of given ContentID.
We should always sync all ContentHeaders of all ContentIDs for a given swarm,
but we may choose to ignore syncing DataChunks for some or all ContentIDs.
ContentIndex is a tool to reconstruct data from pieces in correct order.
A link should be up to 64 bytes long, so 1 byte for DataType,
8 bytes for FounderId, 4 bytes for ContentID, and 51 bytes for SwarmName 
(an UTF-8 String of limited to 51 bytes size).
A house is a link to other user's home/village, whereas other types
of building link to other types of application, like a Forum app for example.
A room within a house, together with floors is used for ordering data. 
Inside a room different objects can be used to represent different kinds of files.
For example a square can represent a picture, just like icons do on a computer system.
Overall this can be remotely representing folders/files on computer's disk.
A user can also use a room as a way to organize information on some specific topic
with interlinked notes containing optional external links (sort of Zettelkasten method).
A house can have up to 32 floors with up to 8 rooms on each floor (u8 value 5|3 split).
A village can cosist of up to 256 streets.
Each street is a representation of a label/tag that given user is interested in.
There can be duplicates of houses on multiple different streets,
but not on the same street.
On every street there can be a Broadcast channel set up used for providing unsynced
data for everyone interested in whatever content it offers.
Later there can also be Multicast channels with access limited to selected visitors.
