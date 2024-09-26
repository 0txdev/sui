module 0x3c7ce8d5655840c294a0897f3ea709c0a83e863d6aa059ca78643ba143c2084::feng {
    struct FENG has drop {
        dummy_field: bool,
    }
    
    fun init(arg0: FENG, arg1: &mut 0x2::tx_context::TxContext) {
        let (v0, v1) = 0x2::coin::create_currency<FENG>(arg0, 6, b"FENG", b"Feng Sui", b"When there's $SUI, there must also be $FENG.", 0x1::option::some<0x2::url::Url>(0x2::url::new_unsafe_from_bytes(b"https://api.movepump.com/uploads/1000000325_374c640b43.gif")), arg1);
        0x2::transfer::public_transfer<0x2::coin::TreasuryCap<FENG>>(v0, 0x2::tx_context::sender(arg1));
        0x2::transfer::public_share_object<0x2::coin::CoinMetadata<FENG>>(v1);
    }
    
    // decompiled from Move bytecode v6
}

