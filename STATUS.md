# Status

| Model                            | Introduction Date | SoC    | Supported | LCD | Audio | RTC | USB | FireWire  | Clickwheel |
| -------------------------------- | ----------------- | ------ | --------- | --- | ----- | --- | --- | --------- | ---------- |
| iPod mini                        | January ‘04       | PP5020 | No        | Yes | No    | WIP | WIP | Won’t Do¹ | ?          |
| iPod (4th Generation)            | July ‘04          | PP5020 | Yes       | Yes | No    | WIP | WIP | Won’t Do¹ | Yes        |
| iPod photo                       | February ‘05      | PP5020 | No        | No  | No    | WIP | WIP | Won’t Do¹ | ?          |
| iPod mini (2nd Generation)       | February ‘05      | PP5022 | No        | Yes | No    | WIP | WIP | Won’t Do¹ | ?          |
| iPod with Color Display          | June ‘05          | PP5020 | No        | No  | No    | WIP | WIP | Won’t Do¹ | ?          |
| iPod Nano                        | September ‘05     | PP5021 | No        | No  | No    | ?   | ?   | Won’t Do¹ | ?          |
| iPod (5th Generation)            | October ‘05       | PP5021 | No        | No  | No    | ?   | ?   | Won’t Do¹ | ?          |
| iPod (5th Generation – Enhanced) | September ‘06     | PP5021 | No        | No  | No    | ?   | ?   | Won’t Do¹ | ?          |

_¹FireWire is slowly being deprecated on modern OS and lacks a cross-platform mechanism for device emulation (like USBIP for USB)._

## Other models

Earlier models (iPod, from 1st Generation to 3rd Generation) could be later added the table. They use PP5002 chips, which have an external FireWire controller, and a slightly different peripheral mapping so they might be added to the table above.

Later iPod Classic and Nano models are based on Samsung S5L chips. The architecture from those chips is radically different from PortalPlayer chips. These models will not be emulated in Clicky.

Last but not least, Shuffle models aren't well suited for emulation.
