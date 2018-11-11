#!/usr/bin/env sh

export ONLINE_API_KEY="<YOUR API KEY HERE>"

#Usage: dns_myapi_add _acme-challenge.www.domain.com  "XKrxpRBosdIKFzxW_CT3KLZNf6q0HG9i01zxXp5CPBs"
dns_online_add() {
    ./le_dns_online add_record "$ONLINE_API_KEY" $1 $2
}

#Usage: dns_myapi_rm _acme-challenge.www.domain.com  "XKrxpRBosdIKFzxW_CT3KLZNf6q0HG9i01zxXp5CPBs"
dns_online_rm() {
    ./le_dns_online delete_record "$ONLINE_API_KEY" $1 $2
}
