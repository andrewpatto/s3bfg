
By default connections to S3 servers are persistent (as per HTTP 1.1).
Note: the addition of *any* keep alive header by the client (as per HTTP 1.0) apparently tells the S3 server to
*not* make the connection persistent - no matter what the value is set to. So if you add a Connection: keep-alive
to your headers the connection *will* be closed immediately after your first operation.

The S3 persistent connections have an extremely short inactivity timeout.
Experimentation puts this timeout at approximately 5 seconds.

Whilst a single connection is capable of transferring many gigabytes of data, each single persistent connection
can perform no more than 100 HTTP GET operations before the connection will be closed. This means that
the size of the objects being fetched (in the case of partial files) will determine how much reuse can be
made out of connections.
