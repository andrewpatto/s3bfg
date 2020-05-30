```
; <<>> DiG 9.11.3-1ubuntu1.12-Ubuntu <<>> @169.254.169.253 sfdsdfsdf.s3.ap-southeast-2.amazonaws.com
; (1 server found)
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 50870
;; flags: qr rd ra; QUERY: 1, ANSWER: 2, AUTHORITY: 0, ADDITIONAL: 1

;; OPT PSEUDOSECTION:
; EDNS: version: 0, flags:; udp: 4096
;; QUESTION SECTION:
;sfdsdfsdf.s3.ap-southeast-2.amazonaws.com. IN A

;; ANSWER SECTION:
sfdsdfsdf.s3.ap-southeast-2.amazonaws.com. 60 IN CNAME s3-r-w.ap-southeast-2.amazonaws.com.
s3-r-w.ap-southeast-2.amazonaws.com. 1 IN A     52.95.134.58

;; Query time: 3 msec
;; SERVER: 169.254.169.253#53(169.254.169.253)
;; WHEN: Mon May 25 03:43:47 UTC 2020
;; MSG SIZE  rcvd: 107
```

```
; <<>> DiG 9.11.4-P2-RedHat-9.11.4-9.P2.amzn2.0.3 <<>> @169.254.169.253 ytyrtyrty.s3.ap-southeast-2.amazonaws.com
; (1 server found)
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 6700
;; flags: qr rd ra; QUERY: 1, ANSWER: 2, AUTHORITY: 0, ADDITIONAL: 0

;; QUESTION SECTION:
;ytyrtyrty.s3.ap-southeast-2.amazonaws.com. IN A

;; ANSWER SECTION:
ytyrtyrty.s3.ap-southeast-2.amazonaws.com. 300 IN CNAME s3-r-w.ap-southeast-2.amazonaws.com.
s3-r-w.ap-southeast-2.amazonaws.com. 0 IN A     52.95.132.122

;; Query time: 7 msec
;; SERVER: 169.254.169.253#53(169.254.169.253)
;; WHEN: Mon May 25 03:44:48 UTC 2020
;; MSG SIZE  rcvd: 96
```
