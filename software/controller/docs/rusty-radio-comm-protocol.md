# Rusty Radio Communication Protocol


## Direction of communication 
Send  = From UI processor -> Radio Processor
Response = From Radio Processor -> UI Processor

# Syntax 


## Send commmand

This takes the form;

```
<send-command>  ::=  <command> ":"  <parameter-list> ";"

<<parameter-list> ::= ( <parameter> ( "," <parameter> )* )? ";"

<command> ::= <capital-letter> <capital-letter> <capital-letter>

<capital-letter> ::= A-Z


```

## Response
```
<acknowledge-response> ::= "ACK:" ( <parameter> ( "," <parameter> )* )? ";"

<response> ::= <response_list> | <error-response>

<error-response> ::= "ERR:" <error-code>

<error-code> ::= (0-9)+
```

# Command List

| Command | Parameter-List | Response-List | Example Parameters| Example Response |Notes |
|---------|------------|----------|-------------|------------------|-------|
| CFG | | n | | 12 | Query the configuration status. Returns n - the number of stations |
| PRE | selected preset id |  | 3 |  | Select a preset |
| STA | station-id | The selected station name | 4 | | Command to tune into the station|
| STA | station-id |  error-code | 4 | | Command to tune into the station|









