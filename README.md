# DNS Resolver

Classic DNS Resolver based on RFC 1035 built entirely in Rust

## Resolver Flow

```
Stub Resolver
↓ UDP 53 | TCP 53 | TLS 853
Socket listens
↓
JobQueue
↓
WorkerPool (Crossbeam)
↓
DNS Packet Parser
↓
In-Flight Cache (RwLock)
↓
DNS Cache (RwLock)
↓
Recursive Resolution
↓
Send Response
```

## DNS Query Structure

DNS Query consists of following message structure which is same for request and response:

| DNS Message |
| ----------- |
| Header      |
| Question    |
| Answer      |
| Authority   |
| Additional  |

### Header

| Header (12 bytes)                                    | Size    |
| ---------------------------------------------------- | ------- |
| ID      (16 bits)                                    | 2 bytes |
| Flags: QR, Opcode(4), AA, TC, RD, RA, Z(3), RCODE(4) | 2 bytes |
| QDCOUNT (1 bit)                                      | 2 bytes |
| ANCOUNT (1 bit)                                      | 2 bytes |
| NSCOUNT (1 bit)                                      | 2 bytes |
| ARCOUNT (1 bit)                                      | 2 bytes |

#### ID
16 bit random number matching request to response

#### Flags

##### QR
If this is a query or response
```
0 = query
1 = response
```

##### Opcode
4 bits
```
0000 = Standard Query
0001 = Inverse Query
0002 = Status
```

##### AA
Authoritative Answer, meaningful for responses
```
0 = not authoritative
1 = authoritative
```

##### TC
Truncated, if this response is too large for response using UDP
```
0 = normal
1 = truncated, response too large
```

##### RD
Recursion Desired for this query
```
0 = not desired
1 = desired, resolve query recursively
```

##### RA
Recursion Available
```
1 = I support recursion
```

##### Z
3 reserved bits, must be 000

##### RCODE
Response Code

```
0 No error
1 Format error
2 Server failure
3 NXDOMAIN
4 Not implemented
5 Refused
```

#### QDCOUNT
Question Count, number of questions in this query, typically 1

#### ANCOUNT
Number of answers

#### NSCOUNT
Authority Records

#### ARCOUNT
Additional Records

#### Example Header

```
ID = 0x1234

Flags

QR = 0
Opcode = 0
AA = 0
TC = 0
RD = 1
RA = 0
Z = 0
RCODE = 0

QDCOUNT = 1
ANCOUNT = 0
NSCOUNT = 0
ARCOUNT = 0
```

### Question Section

QNAME: Query Message, like google.com (formatted)  
QTYPE: Type of record, like A, AAAA, MX  
QCLASS: Record class, usually IN

#### QNAME

Format is as below:

`google.com` is represented as:

| Size | Label    |
| ---- | -------- |
| 6    | google   |
| 3    | com      |
| 0    | root (.) |

#### QTYPE
2 bytes representing record type

```
1   A
2   NS
5   CNAME
15  MX
16  TXT
28  AAAA
```

#### QCLASS
Record class, usually IN (1)

#### Example Question Section

Query
```
google.com A
```

Question:
```
06 google
03 com
00

0001

0001
```

### Answer Section

NAME: Query name, like google.com, similar to QNAME  
TYPE: Record type  
CLASS: Record class  
TTL: 32-bit int, representing time to live  
RDLENGTH: Length of response in bytes, 4 for IPv4, 16 for IPv6  
RDATA: Response data in bytes

### Authority Section

Same Resource Records format  
Contains NS records, delegating resolution to another server

Example:
```
google.com    NS    ns1.google.com
```

### Additional Section
Also Resource Records.

Contains useful extras like glue records for authority section

Example:
```
ns1.google.com

A

216.239.x.x
```

## DNS Name Compression

DNS Query uses domain names which repeats many time accross sections, to avoid such duplicacy, it uses pointers

### Pointer

Pointer Format:
```
11xxxxxx xxxxxxxx
```

Top two bits are 11, denoting its a pointer, remaining bits are offset from the start of the DNS message.

Example:
```
HX:   C0 0C
Bits: 11000000 00001100

Offset: 0x000C
```

