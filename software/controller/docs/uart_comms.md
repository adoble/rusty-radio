## Commands

These commands are send over the uart to the display processor.


TUN:{delta},{range};

Move the tuning dial by delta.


STN:{station name};

Set the station name

PRE:{preset number},{station name};

Set a preset

WFI:{connected};

Indicatd if the wifi is connected. 0 = not connected, 1 = connected

BAT:level;


Indicate that the battery is low. Level is 0 (empty) or 1 (full)

ERR:{severity}, {error message}

An error has occured. 


