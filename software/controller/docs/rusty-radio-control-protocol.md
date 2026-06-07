# Rusty Radio Communication Protocol


## Direction of communication 
Send  = From UI processor -> Radio Processor
Response = From Radio Processor -> UI Processor

# Syntax 

## Framing Grammer
This acts has a template for all application specific commands and responses.

### Send commmand

This takes the form;


```ebnf
<send_command>   ::= <command> ":" <parameter_list> ";"
<parameter_list> ::= ( <parameter> ( <separator>  <parameter> )* )?
<command>        ::= <capital_letter> <capital_letter> <capital_letter>
<parameter>      ::= <utf8_string>
<capital_letter> ::= [A-Z]
<separator>      ::= ","
<positive_integer> ::= <digit>+
<digit>          ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
```

## Response
```ebnf

<response>  ::= <acknowledge_response>  | <error_response>

<acknowledge_response> ::= "ACK:"  <parameter_list> ";"

<error_response> ::= "ERR:" <error_code> ";"

<error_code> ::= <positive_integer>

```

## General  

```ebnf

<utf8_string>   ::=  <utf8_char>*

<utf8_char> ::= /* A set of UTF8_1 characters */ 
```


## Application specific commands


### Query configuation (CFG)

```bnf

;;; <query_config_command> has no parameters
<query_config_command>  ::= "CFG" ":" ";"
<query_config_response> ::= "ACK" ":" <number_stations> "," <presets> ";"
<number_stations>       ::= <positive_integer>
<presets>               ::= ( <preset> ( "," <preset> )* )?
<preset>                ::= <positive_integer>

```
### Selecting a station

```bnf

;;; If station_id is not spectified then the radio is detuned/muted
<station_select_command> ::= "STA" ";" (<station_id> |  ";")
<station_select_response> ::= "ACK" ":" <station_name> ";"
<station_name> ::= UTF8_CHARACTER* 
    
    

```






