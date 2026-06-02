# Rusty Radio Communication Protocol


## Direction of communication 
Send  = From UI processor -> Radio Processor
Response = From Radio Processor -> UI Processor

# Syntax 

## Framing Grammer
This acts has a template for all application specific commands and responses.

### Send commmand

This takes the form;

```bnf
<send-command>   ::= <command> ":" <parameter-list> ";"
<parameter-list> ::= ( <parameter> ( <separator>  <parameter> )* )?
<command>        ::= <capital-letter> <capital-letter> <capital-letter>
<capital-letter> ::= "A"-"Z"
<separator>      ::= ","
<positive-integer> ::= <digit>+
<digit>          ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"

```

### Response

``` bnf

<response>  ::= <acknowledge-response>  | <error-response>
<acknowledge-response> ::= "ACK:"  <parameter-list> ";"

<response> ::= <response_list> | <error-response>

<error-response> ::= "ERR:" <error-code> ";"

<error-code> ::= <positive-integer>
```



## Application specific commands


### Query configuation (CFG)

```bnf

;;; <query-config-command> has no parameters
<query-config-command>  ::= "CFG" ":" ";"
<query-config-response> ::= "ACK" ":" <number-stations> "," <presets> ";"
<number-stations>       ::= <positive-integer>
<presets>               ::= ( <preset> ( "," <preset> )* )?
<preset>                ::= <positive-integer>

```
### Selecting a station

```bnf

;;; If station-id is not spectified then the radio is detuned/muted
<station-select-command> ::= "STA" ";" (<station-id> |  ";")
<station-select-response> ::= "ACK" ":" <station-name> ";"
<station-name> ::= UTF8-CHARACTER* 
    
    

```






